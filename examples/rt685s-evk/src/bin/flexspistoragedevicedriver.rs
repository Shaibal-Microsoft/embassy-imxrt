#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_imxrt::flexspistorage::{
    AhbConfig, FlexSpiBusWidth, FlexSpiFlashPort, FlexSpiFlashPortDeviceInstance, FlexspiAhbBufferConfig,
    FlexspiAhbWriteWaitUnit, FlexspiConfig, FlexspiCsIntervalCycleUnit, FlexspiDeviceConfig, FlexspiReadSampleClock,
    FlexspiStorage,
};
use embassy_imxrt::spinorstorage::SpiStorage;
use embassy_imxrt::storage::{
    BlockingNorStorageDriver, ConfigureCmdSeq, NorStorageCmd, NorStorageCmdMode, NorStorageCmdSeq, NorStorageCmdType,
};
use embassy_time::Timer;
use embedded_storage::nor_flash::{NorFlash as BlockingNorFlash, ReadNorFlash as BlockingReadNorFlash};
use {defmt_rtt as _, panic_probe as _};

static ADDR: u32 = 0x2F000;

struct StorageDeviceDriver {
    // Bus driver dependency
    spi_nor_storage_bus: Option<SpiStorage<embassy_imxrt::spinorstorage::Blocking>>,
    flexspi_nor_storage_bus: Option<FlexspiStorage<embassy_imxrt::flexspistorage::Blocking>>,
}

impl StorageDeviceDriver {
    pub fn new(
        spidriver: Option<SpiStorage<embassy_imxrt::spinorstorage::Blocking>>,
        flexspidriver: Option<FlexspiStorage<embassy_imxrt::flexspistorage::Blocking>>,
    ) -> Result<Self, ()> {
        if let Some(spi) = spidriver {
            return Ok(StorageDeviceDriver {
                spi_nor_storage_bus: Some(spi),
                flexspi_nor_storage_bus: None,
            });
        };
        if let Some(flexspi) = flexspidriver {
            return Ok(StorageDeviceDriver {
                spi_nor_storage_bus: None,
                flexspi_nor_storage_bus: Some(flexspi),
            });
        }

        Err(())
    }

    pub fn init(&self) {
        let bus_ref = self.flexspi_nor_storage_bus.as_ref().unwrap();
        let cmdarr = NorStorageCmdSeq {
            fast_read: Some(NorStorageCmd {
                cmd_lb: 0xEE,
                cmd_ub: Some(0x11),
                addr_width: Some(4),
                mode: NorStorageCmdMode::DDR,
                dummy: Some(20),
                cmdtype: Some(NorStorageCmdType::Read),
            }),
            page_program: Some(NorStorageCmd {
                cmd_lb: 0x12,
                cmd_ub: Some(0xED),
                addr_width: Some(4),
                mode: NorStorageCmdMode::DDR,
                dummy: None,
                cmdtype: Some(NorStorageCmdType::Write),
            }),
            sector_erase: Some(NorStorageCmd {
                cmd_lb: 0x21,
                cmd_ub: Some(0xDE),
                addr_width: Some(4),
                mode: NorStorageCmdMode::DDR,
                dummy: None,
                cmdtype: None,
            }),
            write_enable: Some(NorStorageCmd {
                cmd_lb: 0x06,
                cmd_ub: Some(0xF9),
                addr_width: None,
                mode: NorStorageCmdMode::DDR,
                dummy: None,
                cmdtype: None,
            }),
            write_disable: None,
            read_id: Some(NorStorageCmd {
                cmd_lb: 0x9F,
                cmd_ub: Some(0x60),
                addr_width: Some(4),
                mode: NorStorageCmdMode::DDR,
                dummy: None,
                cmdtype: Some(NorStorageCmdType::Read),
            }),
            poweup: None,
            powerdonw: None,
            read_status_reg: Some(NorStorageCmd {
                cmd_lb: 0x05,
                cmd_ub: Some(0xFA),
                addr_width: Some(4),
                mode: NorStorageCmdMode::DDR,
                dummy: Some(4),
                cmdtype: Some(NorStorageCmdType::Read),
            }),
            write_status_reg: None,
            read_cfg_reg1: None,
            write_cfg_reg1: None,
            read_cfg_reg2: None,
            write_cfg_reg2: None,
            read_cfg_reg3: None,
            write_cfg_reg3: None,
        };

        // Register the Cmd table with FlexSPI Storage
        bus_ref.configure_cmd_seq(&cmdarr);
    }

    pub fn read(&mut self, addr: u32, data: &mut [u8]) {
        let bus_ref = self.flexspi_nor_storage_bus.as_mut().unwrap();
        // Read data from the storage device
        bus_ref.read(addr as u32, data);
    }

    pub fn write(&mut self, addr: u32, data: &[u8]) {
        let bus_ref = self.flexspi_nor_storage_bus.as_mut().unwrap();
        // Write data to the storage device
        bus_ref.write_enable();

        bus_ref.erase(addr, addr + data.len() as u32);

        bus_ref.write_enable();

        let data_size = data.len();
        for i in 0..data_size {
            bus_ref.write(addr + i as u32, &data);
            bus_ref.write_enable();
        }
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_imxrt::init(Default::default());

    // Consider this is a storage service or file system service
    // As per the design, this service is supposed to instantiate low level bus object and configure the bus driver
    // and pass it to the storage device driver when creating it as a dependency injection
    // Bus drivers -
    //      1. FlexspiStorage
    //      2. SpiStorage

    let mut read_data = [0_u8; 32];
    let mut write_data = [0_u8; 32];
    let flash_config = FlexspiDeviceConfig {
        flexspi_root_clk: 48000000,
        is_sck2_enabled: false,
        // Flash size in this struct is in KB, so divide by 1KB
        flash_size_kb: 0x10000, // 64 MB
        cs_interval_unit: FlexspiCsIntervalCycleUnit::CsIntervalUnit1Cycle,
        cs_interval: 2,
        cs_hold_time: 3,
        cs_setup_time: 3,
        data_valid_time: 2,
        columnspace: 0,
        enable_word_address: false,
        awr_seq_index: 1,
        awr_seq_number: 0,
        ard_seq_index: 0,
        ard_seq_number: 0,
        ahb_write_wait_unit: FlexspiAhbWriteWaitUnit::FlexspiAhbWriteWaitUnit2ahbCycle,
        ahb_write_wait_interval: 0,
        enable_write_mask: false,
    };
    let ahb_buffer_config = FlexspiAhbBufferConfig {
        priority: 0,
        master_index: 0,
        buffer_size: 256,
        enable_prefetch: true,
    };

    let ahb_config = AhbConfig {
        enable_ahb_write_ip_rx_fifo: false,
        enable_ahb_write_ip_tx_fifo: false,
        ahb_grant_timeout_cycle: 0xff,
        ahb_bus_timeout_cycle: 0xffff,
        resume_wait_cycle: 0x20,
        buffer: [ahb_buffer_config; 8],
        enable_clear_ahb_buffer_opt: false,
        enable_read_address_opt: true,
        enable_ahb_prefetch: true,
        enable_ahb_bufferable: true,
        enable_ahb_cachable: true,
    };

    let flexspi_config = FlexspiConfig {
        rx_sample_clock: FlexspiReadSampleClock::FlexspiReadSampleClkLoopbackInternally,
        enable_sck_free_running: false,
        enable_combination: false,
        enable_doze: false, // TODO - Check back after analyzing system low power mode requirements
        enable_half_speed_access: false,
        enable_sck_b_diff_opt: false,
        enable_same_config_for_all: false,
        seq_timeout_cycle: 0xFFFF,
        ip_grant_timeout_cycle: 0xff,
        tx_watermark: 0x08,
        rx_watermark: 0x08,
        ahb_config,
    };

    let mut flexspi_storage = FlexspiStorage::new_blocking(
        p.FLEXSPI,       // FlexSPI peripheral
        Some(p.PIO1_11), // FlexSPI DATA 0 pin
        Some(p.PIO1_12),
        Some(p.PIO1_13),
        Some(p.PIO1_14),
        Some(p.PIO2_17),
        Some(p.PIO2_18),
        Some(p.PIO2_22),
        Some(p.PIO2_23),
        p.PIO1_29,
        p.PIO2_19,
        FlexSpiFlashPort::PortB,                         // FlexSPI port
        FlexSpiBusWidth::Octal,                          // FlexSPI bus width
        FlexSpiFlashPortDeviceInstance::DeviceInstance0, // FlexSPI device instance
    );

    flexspi_storage.configport.configure_flexspi(&flexspi_config); // Configure the Flexspi controller

    flexspi_storage
        .configport
        .configure_flexspi_device(&flash_config, &flexspi_config); // Configure the Flash device specific parameters like CS time, etc

    // Instanctiate the storage device driver and inject the bus driver dependency
    let mut device_driver = StorageDeviceDriver::new(None, Some(flexspi_storage)).unwrap();
    device_driver.init();

    // write data
    device_driver.write(ADDR, &write_data);

    device_driver.read(ADDR, &mut read_data);

    loop {
        Timer::after_millis(2000).await;
    }
}

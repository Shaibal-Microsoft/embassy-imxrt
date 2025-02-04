#![no_std]
#![no_main]

use core::arch::asm;
use core::panic;

use cortex_m::register::control;
use defmt::*;
use embassy_executor::Spawner;
use embassy_imxrt::flexspi::{
    AhbConfig, FlexSPI, FlexSpiBusWidth, FlexSpiFlashPort, FlexSpiFlashPortDeviceInstance, FlexSpiLutSeq,
    FlexspiAhbBufferConfig, FlexspiAhbWriteWaitUnit, FlexspiConfig, FlexspiCsIntervalCycleUnit, FlexspiDeviceConfig,
    FlexspiReadSampleClock,
};
use embassy_imxrt::gpio::Flex;
use embassy_imxrt::pac::usbhsd::info;
use embassy_time::Timer;
use embedded_storage::nor_flash::{NorFlash as BlockingNorFlash, ReadNorFlash as BlockingReadNorFlash};
use embedded_storage_async::nor_flash::{
    NorFlash as AsyncNorFlash, NorFlashError, NorFlashErrorKind, ReadNorFlash as AsyncReadNorFlash,
};
use mimxrt600_fcb::flexspi_lut_seq;
use mimxrt600_fcb::FlexSpiLutOpcode::*;
use mimxrt600_fcb::FlexSpiNumPads::*;
use {defmt_rtt as _, panic_probe as _};

extern "C" {
    static mut __region_app1_destination_ram_start_addr__: u8;
    static mut __region_app1_destination_ram_end_addr__: u8;
    static mut __region_app1_src_flash_start_addr__: u8;
    static mut __region_app1_src_flash_end_addr__: u8;
}

#[repr(C)]
struct FlexSpi {
    MCR0: u32,
    /**< Module Control Register 0, offset: 0x0 */
    MCR1: u32,
    /**< Module Control Register 1, offset: 0x4 */
    MCR2: u32,
    /**< Module Control Register 2, offset: 0x8 */
    AHBCR: u32,
    /**< AHB Bus Control Register, offset: 0xC */
    INTEN: u32,
    /**< Interrupt Enable Register, offset: 0x10 */
    INTR: u32,
    /**< Interrupt Register, offset: 0x14 */
    LUTKEY: u32,
    /**< LUT Key Register, offset: 0x18 */
    LUTCR: u32,
    /**< LUT Control Register, offset: 0x1C */
    AHBRXBUFCR0: [u32; 8],
    /**< AHB RX Buffer 0 Control Register 0..AHB RX Buffer 7 Control Register 0, array offset: 0x20, array step: 0x4 */
    RESERVED_0: [u8; 32],
    FLSHCR0: [u32; 4],
    /**< Flash Control Register 0, array offset: 0x60, array step: 0x4 */
    FLSHCR1: [u32; 4],
    /**< Flash Control Register 1, array offset: 0x70, array step: 0x4 */
    FLSHCR2: [u32; 4],
    /**< Flash Control Register 2, array offset: 0x80, array step: 0x4 */
    RESERVED_1: [u8; 4],
    FLSHCR4: u32,
    /**< Flash Control Register 4, offset: 0x94 */
    RESERVED_2: [u8; 8],
    pub IPCR0: u32,
    /**< IP Control Register 0, offset: 0xA0 */
    IPCR1: u32,
    /**< IP Control Register 1, offset: 0xA4 */
    RESERVED_3: [u8; 8],
    IPCMD: u32,
    /**< IP Command Register, offset: 0xB0 */
    DLPR: u32,
    /**< Data Learn Pattern Register, offset: 0xB4 */
    IPRXFCR: u32,
    /**< IP RX FIFO Control Register, offset: 0xB8 */
    IPTXFCR: u32,
    /**< IP TX FIFO Control Register, offset: 0xBC */
    DLLCR: [u32; 2],
    /**< DLL Control Register 0, array offset: 0xC0, array step: 0x4 */
    RESERVED_4: [u8; 24],
    STS0: u32,
    /**< Status Register 0, offset: 0xE0 */
    STS1: u32,
    /**< Status Register 1, offset: 0xE4 */
    STS2: u32,
    /**< Status Register 2, offset: 0xE8 */
    AHBSPNDSTS: u32,
    /**< AHB Suspend Status Register, offset: 0xEC */
    IPRXFSTS: u32,
    /**< IP RX FIFO Status Register, offset: 0xF0 */
    IPTXFSTS: u32,
    /**< IP TX FIFO Status Register, offset: 0xF4 */
    RESERVED_5: [u8; 8],
    RFDR: [u32; 32],
    /**< IP RX FIFO Data Register 0..IP RX FIFO Data Register 31, array offset: 0x100, array step: 0x4 */
    TFDR: [u32; 32],
    /**< IP TX FIFO Data Register 0..IP TX FIFO Data Register 31, array offset: 0x180, array step: 0x4 */
    LUT: [u32; 120],
}

enum FlexSpiCmd {
    FastReadCustom,
    WriteEnable,
    WriteDisable,
    ReadStatusRegister,
    WriteStatusRegister,
    EraseSector,
    EraseBlock,
    EraseChip,
    ReadId,
    PageProgram,
    ReadConfigRegister2,
    ResetEnable,
    ResetDevice,
    WriteConfiRegister2,
}

#[no_mangle]
#[link_section = ".data"]
fn setup_transfer(controller: &mut FlexSpi, cmd: FlexSpiCmd, addr: Option<u32>, data: Option<u32>, size: Option<u32>) {
    match addr {
        Some(addr) => unsafe {
            controller.IPCR0 = addr;
        },
        None => unsafe {
            controller.IPCR0 = 0;
        },
    }
    // Clear the sequence ID
    controller.IPCR1 &= !(0x1F << 16);

    // Unlock the LUT
    controller.LUTKEY = 0x5AF05AF0;
    controller.LUTCR = 0x2;

    // Reset the sequence pointer
    controller.FLSHCR2[0] |= 0x1 << 31;
    controller.FLSHCR2[1] |= 0x1 << 31;
    controller.FLSHCR2[2] |= 0x1 << 31;
    controller.FLSHCR2[3] |= 0x1 << 31;

    match cmd {
        FlexSpiCmd::ReadId => {
            // Working partially
            controller.IPCR1 |= 11 << 16;
            controller.LUT[4 * 11] = flexspi_lut_seq(CMD_DDR, Octal, 0x9F, CMD_DDR, Octal, 0x60);
            controller.LUT[4 * 11 + 1] = flexspi_lut_seq(RADDR_DDR, Octal, 0x20, DUMMY_DDR, Octal, 0x29);
            controller.LUT[4 * 11 + 2] = flexspi_lut_seq(READ_SDR, Octal, 0x3, STOP, Single, 0x0);
            controller.LUT[4 * 11 + 3] = 0;
        }
        FlexSpiCmd::FastReadCustom => {
            // Working
            controller.IPCR1 |= (6 << 16);

            controller.LUT[4 * 6] = flexspi_lut_seq(CMD_DDR, Octal, 0xee, CMD_DDR, Octal, 0x11);
            controller.LUT[4 * 6 + 1] = flexspi_lut_seq(RADDR_DDR, Octal, 0x20, DUMMY_DDR, Octal, 0x29);
            controller.LUT[4 * 6 + 2] = flexspi_lut_seq(READ_DDR, Octal, 0x04, STOP, Single, 0x00);
            controller.LUT[4 * 6 + 3] = 0;
        }
        FlexSpiCmd::WriteEnable => {
            // Working
            controller.IPCR1 |= 14 << 16;
            controller.LUT[4 * 14] = flexspi_lut_seq(CMD_DDR, Octal, 0x06, CMD_DDR, Octal, 0xF9);
            controller.LUT[4 * 14 + 1] = 0;
            controller.LUT[4 * 14 + 2] = 0;
            controller.LUT[4 * 14 + 3] = 0;
        }
        FlexSpiCmd::ReadStatusRegister => {
            // Working
            controller.IPCR1 |= 15 << 16;
            controller.LUT[4 * 15] = flexspi_lut_seq(CMD_DDR, Octal, 0x05, CMD_DDR, Octal, 0xFA);
            controller.LUT[4 * 15 + 1] = flexspi_lut_seq(RADDR_DDR, Octal, 0x20, DUMMY_DDR, Octal, 0x14);
            controller.LUT[4 * 15 + 2] = flexspi_lut_seq(READ_SDR, Octal, 0x4, STOP, Single, 0x0);
            controller.LUT[4 * 15 + 3] = 0;
        }
        FlexSpiCmd::PageProgram => {
            controller.IPCR1 |= 16 << 16;
            controller.LUT[4 * 16] = flexspi_lut_seq(CMD_DDR, Octal, 0x12, CMD_DDR, Octal, 0xED);
            controller.LUT[4 * 16 + 1] = flexspi_lut_seq(RADDR_DDR, Octal, 0x20, WRITE_DDR, Octal, 0x4);
            controller.LUT[4 * 16 + 2] = 0;
            controller.LUT[4 * 16 + 3] = 0;
        }
        FlexSpiCmd::EraseSector => {
            controller.IPCR1 |= 17 << 16;
            controller.LUT[4 * 17] = flexspi_lut_seq(CMD_SDR, Single, 0x20, RADDR_SDR, Single, 0x18);
            controller.LUT[4 * 17 + 1] = 0;
            controller.LUT[4 * 17 + 2] = 0;
            controller.LUT[4 * 17 + 3] = 0;
        }
        FlexSpiCmd::WriteDisable => {
            controller.IPCR1 |= 18 << 16;
            controller.LUT[4 * 18] = flexspi_lut_seq(CMD_DDR, Octal, 0x04, CMD_DDR, Octal, 0xFB);
            controller.LUT[4 * 18 + 1] = 0;
            controller.LUT[4 * 18 + 2] = 0;
            controller.LUT[4 * 18 + 3] = 0;
        }
        FlexSpiCmd::ReadConfigRegister2 => {
            controller.IPCR1 |= 19 << 16;
            controller.LUT[4 * 19] = flexspi_lut_seq(CMD_DDR, Octal, 0x71, CMD_DDR, Octal, 0x8E);
            controller.LUT[4 * 19 + 1] = flexspi_lut_seq(RADDR_DDR, Octal, 0x20, DUMMY_DDR, Octal, 0x4);
            controller.LUT[4 * 19 + 2] = flexspi_lut_seq(READ_SDR, Octal, 0x4, STOP, Single, 0x0);
            controller.LUT[4 * 19 + 3] = 0;
        }
        FlexSpiCmd::ResetEnable => {
            controller.IPCR1 |= 20 << 16;
            controller.LUT[4 * 20] = flexspi_lut_seq(CMD_DDR, Octal, 0x66, CMD_DDR, Octal, 0x99);
            controller.LUT[4 * 20 + 1] = 0;
            controller.LUT[4 * 20 + 2] = 0;
            controller.LUT[4 * 20 + 3] = 0;
        }
        FlexSpiCmd::ResetDevice => {
            controller.IPCR1 |= 21 << 16;
            controller.LUT[4 * 21] = flexspi_lut_seq(CMD_DDR, Octal, 0x99, CMD_DDR, Octal, 0x66);
            controller.LUT[4 * 21 + 1] = 0;
            controller.LUT[4 * 21 + 2] = 0;
            controller.LUT[4 * 21 + 3] = 0;
        }
        FlexSpiCmd::WriteConfiRegister2 => {
            controller.IPCR1 |= 22 << 16;
            controller.LUT[4 * 22] = flexspi_lut_seq(CMD_DDR, Octal, 0x72, CMD_DDR, Octal, 0x8D);
            controller.LUT[4 * 22 + 1] = flexspi_lut_seq(RADDR_DDR, Octal, 0x20, WRITE_DDR, Octal, 0x4);
            controller.LUT[4 * 22 + 2] = 0;
            controller.LUT[4 * 22 + 3] = 0;
        }
        _ => {}
    }

    // Disable DMA for TX and RX
    controller.IPRXFCR &= !0x2;
    controller.IPTXFCR &= !0x2;

    // set watermark
    //controller.IPRXFCR &= !(0x3F << 2);
    //controller.IPRXFCR != (((0x8 / 8) - 1) << 2); // 8 bytes watermark

    // Reset RX and TX FIFO
    controller.IPRXFCR |= 0x1;
    controller.IPTXFCR |= 0x1;
}

#[no_mangle]
#[link_section = ".data"]
fn start_transfer(controller: &mut FlexSpi) {
    controller.IPCMD |= 0x1;
}

#[no_mangle]
#[link_section = ".data"]
fn wait_for_cmd_completion(controller: &mut FlexSpi) {
    while (controller.INTR & 0x1 == 0) {}
}

#[no_mangle]
#[link_section = ".data"]
fn read_cmd_data(controller: &mut FlexSpi, size: u32) -> [u32; 32] {
    let mut data = [0; 32];
    loop {
        //info!("Waiting for filled data = {:02X}", controller.IPRXFSTS & 0xFF);
        if ((controller.IPRXFSTS & 0xFF) * 8) < size {
            continue;
        }
        break;
    }

    data[0] = controller.RFDR[0];
    data[1] = controller.RFDR[1];

    data
}

#[no_mangle]
#[link_section = ".data"]
fn write_cmd_data(controller: &mut FlexSpi, data: [u32; 32], size: u32) {
    let mut loopcnt = size / 4;

    for i in 0..loopcnt {
        controller.TFDR[i as usize] = data[i as usize];
    }

    controller.INTR |= (0x1 << 6);
}

#[no_mangle]
#[link_section = ".data"]
fn do_transaction() {
    let raw_ptr = 0x40134000 as *mut FlexSpi;

    let FlexSPI_ref = unsafe { raw_ptr.as_mut().unwrap() };

    FlexSPI_ref.FLSHCR2[2] |= 0x10;

    // Read Configuration 2 register
    // setup_transfer(FlexSPI_ref, FlexSpiCmd::ReadConfigRegister2, Some(0x0), None, Some(4));
    // start_transfer(FlexSPI_ref);
    // wait_for_cmd_completion(FlexSPI_ref);
    // let data = read_cmd_data(FlexSPI_ref, 4);
    // info!("{:02X}", data[0]);

    FlexSPI_ref.MCR0 |= 0x1;

    while FlexSPI_ref.MCR0 & 0x1 == 0x1 {}

    // Read JEDEC ID
    setup_transfer(FlexSPI_ref, FlexSpiCmd::ReadId, None, None, Some(4));
    start_transfer(FlexSPI_ref);
    wait_for_cmd_completion(FlexSPI_ref);
    let data = read_cmd_data(FlexSPI_ref, 4);
    info!("{:08X}", data[0]);

    // Read Status Register
    setup_transfer(FlexSPI_ref, FlexSpiCmd::ReadStatusRegister, None, None, Some(4));
    start_transfer(FlexSPI_ref);
    wait_for_cmd_completion(FlexSPI_ref);
    let data = read_cmd_data(FlexSPI_ref, 4);
    info!("{:08X}", data[0]);

    // Write Enable
    setup_transfer(FlexSPI_ref, FlexSpiCmd::WriteEnable, None, None, None);
    start_transfer(FlexSPI_ref);
    wait_for_cmd_completion(FlexSPI_ref);

    // Read Status Register
    setup_transfer(FlexSPI_ref, FlexSpiCmd::ReadStatusRegister, None, None, Some(4));
    start_transfer(FlexSPI_ref);
    wait_for_cmd_completion(FlexSPI_ref);
    let data = read_cmd_data(FlexSPI_ref, 4);
    info!("{:08X}", data[0]);

    // Write Disable
    setup_transfer(FlexSPI_ref, FlexSpiCmd::WriteDisable, None, None, None);
    start_transfer(FlexSPI_ref);
    wait_for_cmd_completion(FlexSPI_ref);

    Timer::after_millis(1);

    // Read Status Register
    setup_transfer(FlexSPI_ref, FlexSpiCmd::ReadStatusRegister, None, None, Some(4));
    start_transfer(FlexSPI_ref);
    wait_for_cmd_completion(FlexSPI_ref);
    let data = read_cmd_data(FlexSPI_ref, 4);
    info!("{:08X}", data[0]);

    // // Erase Sector
    // // setup_transfer(FlexSPI_ref, FlexSpiCmd::EraseSector, Some(0x50000), None, None);
    // // start_transfer(FlexSPI_ref);
    // // wait_for_cmd_completion(FlexSPI_ref);

    // //Read Data
    // setup_transfer(FlexSPI_ref, FlexSpiCmd::FastReadCustom, Some(0x70000), None, Some(4));
    // start_transfer(FlexSPI_ref);
    // wait_for_cmd_completion(FlexSPI_ref);
    // let data = read_cmd_data(FlexSPI_ref, 4);

    // info!("{:08X}", data[0]);

    // // Page Program
    // setup_transfer(
    //     FlexSPI_ref,
    //     FlexSpiCmd::PageProgram,
    //     Some(0x70000),
    //     Some(0x12345678),
    //     Some(4),
    // );
    // start_transfer(FlexSPI_ref);
    // wait_for_cmd_completion(FlexSPI_ref);
    // let mut data = [0; 32];
    // data[0] = 0x12345678;
    // write_cmd_data(FlexSPI_ref, data, 4);

    // loop {
    //     // Read Status Register
    //     setup_transfer(FlexSPI_ref, FlexSpiCmd::ReadStatusRegister, None, None, Some(1));
    //     start_transfer(FlexSPI_ref);
    //     wait_for_cmd_completion(FlexSPI_ref);
    //     let data = read_cmd_data(FlexSPI_ref, 1);
    //     // check if WIP is set or cleared
    //     if data[0] & 0x1 == 0x0 {
    //         break;
    //     }
    // }

    // //Read Data
    // setup_transfer(FlexSPI_ref, FlexSpiCmd::FastReadCustom, Some(0x70000), None, Some(4));
    // start_transfer(FlexSPI_ref);
    // wait_for_cmd_completion(FlexSPI_ref);
    // let data = read_cmd_data(FlexSPI_ref, 4);

    // info!("{:08X}", data[0]);

    FlexSPI_ref.FLSHCR2[2] &= !0x1F;
}

#[embassy_executor::main]
#[no_mangle]
async fn main(_spawner: Spawner) {
    let p = embassy_imxrt::init(Default::default());

    let mut read_data: [u8; 256] = [0; 256];
    let mut write_data: [u8; 256] = [0x55; 256];

    let mut flexspi = FlexSPI::new_blocking(
        p.FLEXSPI,                                       // FlexSPI peripheral
        Some(p.PIO1_11),                                 // Data0
        Some(p.PIO1_12),                                 // Data1
        Some(p.PIO1_13),                                 // Data2
        Some(p.PIO1_14),                                 // Data3
        Some(p.PIO2_17),                                 // Data4
        Some(p.PIO2_18),                                 // Data5
        Some(p.PIO2_22),                                 // Data6
        Some(p.PIO2_23),                                 // Data7
        p.PIO1_29,                                       // SCLK
        p.PIO2_19,                                       // SS0/1
        FlexSpiFlashPort::PortB,                         // FlexSPI port B
        FlexSpiBusWidth::Octal,                          // FlexSPI bus width Octal
        FlexSpiFlashPortDeviceInstance::DeviceInstance0, // FlexSPI flash port device instance 0
    );

    flexspi.cmdport.disable_prefetch();

    do_transaction();

    // let mut raw_ptr = 0x40134000 as *mut FlexSpi;

    // let controller = unsafe { raw_ptr.as_mut().unwrap() };

    // controller.LUTKEY = 0x5AF05AF0;
    // controller.LUTCR = 0x2;

    // controller.IPCR1 |= 11 << 16;
    // controller.LUT[4 * 11] = flexspi_lut_seq(CMD_DDR, Octal, 0x9F, CMD_DDR, Octal, 0x60);
    // controller.LUT[4 * 11 + 1] = flexspi_lut_seq(RADDR_DDR, Octal, 0x20, DUMMY_DDR, Octal, 0x29);
    // controller.LUT[4 * 11 + 2] = flexspi_lut_seq(READ_SDR, Octal, 0x3, STOP, Single, 0x0);
    // controller.LUT[4 * 11 + 3] = 0;

    // let mut raw_ptr = 0x40134000 as *mut FlexSpi;

    // let FlexSPI_ref = unsafe { raw_ptr.as_mut().unwrap() };

    //Reading data through Data Port
    // let result = flexspi.dataport.read(0x1000, &mut read_data); // Read data from FlexSPI

    // match result {
    //     Ok(_) => info!("Read data from FlexSPI"),
    //     Err(FlashError) => {
    //         info!("Failed to read data from FlexSPI due to following reason");
    //         match FlashError.kind() {
    //             NorFlashErrorKind::Other => info!("Other error"),
    //             _ => info!("Unknown error"),
    //         }
    //     }
    // }

    // info!("Data at index {} value {:02X}", 0, read_data[0]);
    // info!("Data at index {} value {:02X}", 1, read_data[1]);
    // info!("Data at index {} value {:02X}", 2, read_data[2]);
    // info!("Data at index {} value {:02X}", 3, read_data[3]);

    // FlexSPI_ref.LUT[4 * 11] = flexspi_lut_seq(CMD_DDR, Octal, 0x9F, CMD_DDR, Octal, 0x60);
    // FlexSPI_ref.LUT[4 * 11 + 1] = flexspi_lut_seq(RADDR_DDR, Octal, 0x20, DUMMY_DDR, Octal, 0x31);
    // FlexSPI_ref.LUT[4 * 11 + 2] = flexspi_lut_seq(READ_DDR, Octal, 0x3, STOP, Single, 0x0);
    // FlexSPI_ref.LUT[4 * 11 + 3] = 0;

    // Reading data through Command Port
    // let data = flexspi.cmdport.fast_read(0x1000, 4); // Read data from FlexSPI

    // info!("Data at index {} value {:02X}", 0, data[0]);

    // let id = flexspi.cmdport.read_id(3); // Read ID from FlexSPI
    // info!("ID: {:08X}", id[0]);

    //flexspi.cmdport.write_enable();

    // let status = flexspi.cmdport.read_status_register();
    // info!("Status register value: {:02X}", status[0]);

    //let result = flexspi.cmdport.erase(0x3FF0000, 0x3FF0FFF); // Erase data from FlexSPI

    //flexspi.cmdport.write_enable();

    //let result = flexspi.dataport.write(0x3FF0000, &write_data); // Write data to FlexSPI
    //flexspi.cmdport.write_data(0x3FF0000, &mut write_data, 4);

    //let data = flexspi.cmdport.fast_read(0x3FD0000, 4); // Read data from FlexSPI

    //info!("Data at index {} value {:02X}", 0, data[0]);

    // let result = flexspi.dataport.read(0x3FF0000, &mut read_data); // Read data from FlexSPI

    // for i in 0..256 {
    //     info!("Data at index {} value {}", i, read_data[i]);
    // }

    // for i in 0..10 {
    //     if read_data[i] == write_data[i] {
    //         info!("Data match at index {} value {}", i, read_data[i]);
    //     } else {
    //         info!("Data mismatch at index {}", i);
    //     }
    // }
    loop {
        // info!("Flexspi task running");
        Timer::after_millis(1000).await;
    }
}

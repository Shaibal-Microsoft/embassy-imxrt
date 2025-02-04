#![no_std]

use bitfield::bitfield;
use FlexSpiLutOpcode::*;
use FlexSpiNumPads::*;
use {defmt_rtt as _, panic_probe as _};

// auto-generated version information from Cargo.toml
include!(concat!(env!("OUT_DIR"), "/biv.rs"));

pub struct FlexSPIFlashConfigurationBlock {
    tag: u32,
    version: u32,
    reserved0: [u8; 4],
    pub read_sample_clk: ReadSampleClk,
    pub cs_hold_time: u8,
    pub cs_setup_time: u8,
    pub column_address_width: ColumnAddressWidth,
    pub device_mode_cfg_enable: u8,
    reserved1: [u8; 1],
    pub wait_time_cfg_commands: u16,
    pub device_mode_seq: DeviceModeSeq,
    pub device_mode_arg: [u8; 4],
    pub config_cmd_enable: u8,
    pub config_mode_type: [u8; 3],
    pub config_cmd_seqs: [u8; 12],
    reserved2: [u8; 4],
    pub config_cmd_args: [u8; 12],
    reserved3: [u8; 4],
    pub controller_misc_option: ControllerMiscOption,
    device_type: DeviceType,
    pub sflash_pad_type: SFlashPadType,
    pub serial_clk_freq: SerialClkFreq,
    pub lut_custom_seq_enable: u8,
    reserved4: [u8; 8],
    pub sflash_a1_size: u32,
    pub sflash_a2_size: u32,
    pub sflash_b1_size: u32,
    pub sflash_b2_size: u32,
    pub cs_pad_override: u32,
    pub sclk_pad_setting_override: u32,
    pub data_pad_setting_override: u32,
    pub dqs_pad_setting_override: u32,
    pub timeout_in_ms: u32,
    pub command_interval: u32,
    pub data_valid_time: u32,
    pub busy_offset: u16,
    pub busy_bit_polarity: BusyBitPolarity,
    pub lookup_table: [u32; 64],
    pub lut_custom_seq: [u8; 48],
    reserved5: [u8; 16],
    pub page_size: u32,
    pub sector_size: u32,
    pub ipcmd_serial_clk_freq: u8,
    pub is_uniform_block_size: u8,
    pub is_data_order_swapped: u8,
    reserved6: [u8; 1],
    pub serial_nor_type: SerialNORType,
    pub need_exit_no_cmd_mode: u8,
    pub half_clk_for_non_read_cmd: u8,
    pub need_restore_no_cmd_mode: u8,
    pub block_size: u32,
    pub flash_state_ctx: u32,
    reserved7: [u8; 40],
}

#[repr(u8)]
pub enum ReadSampleClk {
    InternalLoopback = 0,
    LoopBackFromDQSPad = 1,
    LoopBackFromSCKPad = 2,
    ExternalDQSSignal = 3,
}

#[repr(u8)]
pub enum ColumnAddressWidth {
    Other = 0,
    HyperFlash = 3,
}

#[repr(C)]
#[repr(packed)]
pub struct DeviceModeSeq {
    num_seq: u8,
    seq_index: u8,
    reserved: [u8; 2],
}

impl DeviceModeSeq {
    pub const fn build() -> Self {
        Self {
            num_seq: 1,
            seq_index: 6,
            reserved: [0; 2],
        }
    }
}

bitfield! {
    pub struct ControllerMiscOption(u32);
    impl Debug;

    pub differential_clock, set_differential_clock: 0;
    pub word_addressable, set_word_addressable: 3;
    pub safe_config_freq, set_safe_config_freq: 4;
    pub ddr_mode, set_ddr_mode: 6;
}

#[repr(u8)]
enum DeviceType {
    SerialNOR = 1,
}

#[repr(u8)]
pub enum SFlashPadType {
    SinglePad = 1,
    DualPads = 2,
    QuadPads = 4,
    OctalPads = 8,
}

#[repr(u8)]
pub enum SerialClkFreq {
    SdrDdr30mhz = 1,
    SdrDdr50mhz = 2,
    SdrDdr60mhz = 3,
    SdrDdr80mhz = 4,
    SdrDdr100mhz = 5,
    SdrDdr120mhz = 6,
    SdrDdr133mhz = 7,
    SdrDdr166mhz = 8,
    SdrDdr200mhz = 9,
}

#[repr(u16)]
pub enum BusyBitPolarity {
    Normal = 0,
    Inverted = 1,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum FlexSpiLutOpcode {
    CMD_SDR = 0x01,
    CMD_DDR = 0x21,
    RADDR_SDR = 0x02,
    RADDR_DDR = 0x22,
    CADDR_SDR = 0x03,
    CADDR_DDR = 0x23,
    MODE1_SDR = 0x04,
    MODE1_DDR = 0x24,
    MODE2_SDR = 0x05,
    MODE2_DDR = 0x25,
    MODE4_SDR = 0x06,
    MODE4_DDR = 0x26,
    MODE8_SDR = 0x07,
    MODE8_DDR = 0x27,
    WRITE_SDR = 0x08,
    WRITE_DDR = 0x28,
    READ_SDR = 0x09,
    READ_DDR = 0x29,
    LEARN_SDR = 0x0a,
    LEARN_DDR = 0x2a,
    DATSZ_SDR = 0x0b,
    DATSZ_DDR = 0x2b,
    DUMMY_SDR = 0x0c,
    DUMMY_DDR = 0x2c,
    DUMMY_RWDS_SDR = 0x0d,
    DUMMY_RWDS_DDR = 0x2d,
    JMP_ON_CS = 0x1f,
    STOP = 0x00,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FlexSpiNumPads {
    Single = 0,
    Dual = 1,
    Quad = 2,
    Octal = 3,
}

#[repr(u8)]
pub enum SerialNORType {
    StandardSpi = 0,
    HyperBus = 1,
    Xpi = 2,
    NoCmd = 3,
}

#[inline(always)]
pub const fn flexspi_lut_seq(
    opcode0: FlexSpiLutOpcode,
    num_pads0: FlexSpiNumPads,
    operand0: u8,
    opcode1: FlexSpiLutOpcode,
    num_pads1: FlexSpiNumPads,
    operand1: u8,
) -> u32 {
    let opcode0_raw = ((opcode0 as u32) << 10) & 0xfc00;
    let num_pads0_raw = ((num_pads0 as u32) << 8) & 0x0300;
    let operand0_raw = ((operand0 as u32) << 0) & 0xff;
    let opcode1_raw = ((opcode1 as u32) << 26) & 0xfc000000;
    let num_pads1_raw = ((num_pads1 as u32) << 24) & 0x3000000;
    let operand1_raw = ((operand1 as u32) << 16) & 0xff0000;

    opcode0_raw | num_pads0_raw | operand0_raw | opcode1_raw | num_pads1_raw | operand1_raw
}

impl FlexSPIFlashConfigurationBlock {
    const TAG: u32 = 0x42464346;
    const VERSION: u32 = 0x56020000;
    pub const fn build() -> Self {
        Self {
            tag: Self::TAG,
            version: Self::VERSION,
            reserved0: [0; 4],
            read_sample_clk: ReadSampleClk::InternalLoopback,
            cs_hold_time: 3,
            cs_setup_time: 3,
            column_address_width: ColumnAddressWidth::Other,
            device_mode_cfg_enable: 0,
            reserved1: [2; 1],
            wait_time_cfg_commands: 1,
            device_mode_seq: DeviceModeSeq::build(),
            device_mode_arg: [2, 0, 0, 0],
            config_cmd_enable: 0,
            config_mode_type: [0; 3],
            config_cmd_seqs: [0; 12],
            reserved2: [0; 4],
            config_cmd_args: [0; 12],
            reserved3: [0; 4],
            controller_misc_option: ControllerMiscOption(0x50),
            device_type: DeviceType::SerialNOR,
            sflash_pad_type: SFlashPadType::OctalPads,
            serial_clk_freq: SerialClkFreq::SdrDdr50mhz,
            lut_custom_seq_enable: 0,
            reserved4: [0; 8],
            sflash_a1_size: 0,
            sflash_a2_size: 0,
            sflash_b1_size: 0x04000000,
            sflash_b2_size: 0,
            cs_pad_override: 0,
            sclk_pad_setting_override: 0,
            data_pad_setting_override: 0,
            dqs_pad_setting_override: 0,
            timeout_in_ms: 0,
            command_interval: 0,
            data_valid_time: 0,
            busy_offset: 0,
            busy_bit_polarity: BusyBitPolarity::Normal,
            lookup_table: [
                // Read
                flexspi_lut_seq(FlexSpiLutOpcode::CMD_DDR, Octal, 0xee, CMD_DDR, Octal, 0x11),
                flexspi_lut_seq(RADDR_DDR, Octal, 0x20, DUMMY_DDR, Octal, 0x29),
                flexspi_lut_seq(READ_DDR, Octal, 0x04, STOP, Single, 0x00),
                0,
                // Read status SPI
                flexspi_lut_seq(CMD_SDR, Single, 0x05, READ_SDR, Single, 0x04),
                0,
                0,
                0,
                // Read status OPI
                flexspi_lut_seq(CMD_DDR, Octal, 0x05, CMD_DDR, Octal, 0xFA),
                flexspi_lut_seq(RADDR_DDR, Octal, 0x20, DUMMY_DDR, Octal, 0x14),
                flexspi_lut_seq(READ_DDR, Octal, 0x04, STOP, Single, 0x00),
                0,
                // Write enable
                flexspi_lut_seq(CMD_SDR, Single, 0x06, STOP, Single, 0x00),
                0,
                0,
                0,
                // Write enable - OPI
                flexspi_lut_seq(CMD_DDR, Octal, 0x06, CMD_DDR, Octal, 0xF9),
                0,
                0,
                0,
                // Erase Sector
                flexspi_lut_seq(CMD_DDR, Octal, 0x21, CMD_DDR, Octal, 0xDE),
                flexspi_lut_seq(RADDR_DDR, Octal, 0x20, STOP, Single, 0x00),
                0,
                0,
                // Enable OPI DDR mode
                flexspi_lut_seq(CMD_SDR, Single, 0x72, CMD_SDR, Single, 0x00),
                flexspi_lut_seq(CMD_SDR, Single, 0x00, CMD_SDR, Single, 0x00),
                flexspi_lut_seq(CMD_SDR, Single, 0x00, WRITE_SDR, Single, 0x02),
                0,
                // Unused
                0,
                0,
                0,
                0,
                // Erase block
                flexspi_lut_seq(CMD_DDR, Octal, 0xDC, CMD_DDR, Octal, 0x23),
                flexspi_lut_seq(RADDR_DDR, Octal, 0x20, STOP, Single, 0x00),
                0,
                0,
                // Page program
                flexspi_lut_seq(CMD_DDR, Octal, 0x12, CMD_DDR, Octal, 0xED),
                flexspi_lut_seq(RADDR_DDR, Octal, 0x20, WRITE_DDR, Octal, 0x04),
                0,
                0,
                // Read ID
                flexspi_lut_seq(CMD_SDR, Single, 0x9F, READ_SDR, Single, 0x3),
                0,
                0,
                0,
                // Erase chip
                flexspi_lut_seq(CMD_DDR, Octal, 0x60, CMD_DDR, Octal, 0x9F),
                0,
                0,
                0,
                // Remainder is unused
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            lut_custom_seq: [0; 48],
            reserved5: [0; 16],
            page_size: 256,
            sector_size: 4096,
            ipcmd_serial_clk_freq: 1,
            is_uniform_block_size: 0,
            is_data_order_swapped: 0,
            reserved6: [0; 1],
            serial_nor_type: SerialNORType::Xpi,
            need_exit_no_cmd_mode: 0,
            half_clk_for_non_read_cmd: 0,
            need_restore_no_cmd_mode: 0,
            block_size: 0x10_000,
            flash_state_ctx: 0x07008200,
            reserved7: [0; 40],
        }
    }
}

#[link_section = ".otfad"]
#[used]
static OTFAD: [u8; 256] = [0; 256];

#[rustfmt::skip]
#[link_section = ".fcb"]
#[used]
static FCB:FlexSPIFlashConfigurationBlock = FlexSPIFlashConfigurationBlock::build();

#[link_section = ".keystore"]
#[used]
static KEYSTORE: [u8; 2048] = [0; 2048];

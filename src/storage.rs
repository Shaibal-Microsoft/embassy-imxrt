#[derive(Debug, Copy, Clone, PartialEq)]
/// Storage Mode
pub enum NorStorageCmdMode {
    /// DDR mode for data transfer
    DDR,
    /// SDR mode for data transfer
    SDR,
}
#[derive(Debug, Copy, Clone)]
/// Storage Command Type
pub enum NorStorageCmdType {
    /// Read transfer type
    Read,
    /// Write transfer type
    Write,
}

#[derive(Debug, Copy, Clone)]
/// NOR Storage Command to be passed by NOR based storage device drivers
pub struct NorStorageCmd {
    /// Nor Storage Command lower byte
    pub cmd_lb: u8,
    /// Nor Storage Command upper byte                       
    pub cmd_ub: Option<u8>,
    /// Address of the command
    pub addr: Option<u32>,
    /// Address width in bytes              
    pub addr_width: Option<u8>,
    /// DDR or SDR mode             
    pub mode: NorStorageCmdMode,
    /// Number of Dummy clock cycles. Assuming max 256 dummy cycles beyond which its impractical           
    pub dummy: Option<u8>,
    /// Command type - Reading data or writing data
    pub cmdtype: Option<NorStorageCmdType>,
    /// Bus Width - This represents width in terms of signals
    ///     SPI - 1
    ///     QSPI - 4
    ///     Octal - 8
    ///     I2C - 1
    pub bus_width: u8,
    /// Number of data bytes to be transferred for this command
    /// This size is not valid for data read and write command as its a variable size
    pub data_bytes: Option<u8>,
}

/// Trait for reprensenting NOR Storage Bus Error
pub trait NorStorageBusError {
    fn decode_bus_error(&self);
}

/// Blocking NOR Storage Driver
pub trait BlockingNorStorageBusDriver {
    /// Send Command to the bus
    fn execute_cmd(
        &mut self,
        cmd: NorStorageCmd,
        read_buf: Option<&mut [u8]>,
        write_buf: Option<&[u8]>,
    ) -> Result<(), impl NorStorageBusError>;
}

/// Async NOR Storage Driver
pub trait AsyncNorStorageDriver {
    /// Send Command to the bus
    async fn execute_cmd(&self, cmd: NorStorageCmd, buf: Option<&mut [u8]>) -> Result<(), impl NorStorageBusError>;
}

/// Blocking NAND storage driver
pub trait BlockingNandStorageDriver {}

/// Async NAND storage driver
pub trait AsyncNandStorageDriver {}

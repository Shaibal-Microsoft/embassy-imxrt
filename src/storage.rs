use embedded_storage::nor_flash::{NorFlash as BlockingNorFlash, ReadNorFlash as BlockingReadNorFlash};
use embedded_storage_async::nor_flash::{NorFlash as AsyncNorFlash, ReadNorFlash as AsyncReadNorFlash};

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
    /// Address width in bytes              
    pub addr_width: Option<u8>,
    /// DDR or SDR mode             
    pub mode: NorStorageCmdMode,
    /// Number of Dummy clock cycles. Assuming max 256 dummy cycles beyond which its impractical           
    pub dummy: Option<u8>,
    /// Command type - Reading data or writing data
    pub cmdtype: Option<NorStorageCmdType>,
    /// Number of data bytes to be transferred
    pub data_bytes: Option<u8>,
}

/// NOR Storage Command Array to be passed by NOR based storage device drivers
pub struct NorStorageCmdSeq {
    /// Fast Read Sequence
    pub fast_read: Option<NorStorageCmd>,
    /// Page Program Sequence
    pub page_program: Option<NorStorageCmd>,
    /// Sector Erase Sequence
    pub sector_erase: Option<NorStorageCmd>,
    /// Write Enable Sequence
    pub write_enable: Option<NorStorageCmd>,
    /// Write Disable Sequence
    pub write_disable: Option<NorStorageCmd>,
    /// Read JEDEC Id Down Sequence
    pub read_id: Option<NorStorageCmd>,
    /// Power Up Sequence
    pub poweup: Option<NorStorageCmd>,
    /// Power Down Sequence
    pub powerdonw: Option<NorStorageCmd>,
    /// Read Status Register Sequence
    pub read_status_reg: Option<NorStorageCmd>,
    /// Write Status Register Sequence
    pub write_status_reg: Option<NorStorageCmd>,
    /// Read Config1 Register Sequence
    pub read_cfg_reg1: Option<NorStorageCmd>,
    /// Write Config1 Register Sequence
    pub write_cfg_reg1: Option<NorStorageCmd>,
    /// Read Config2 Register Sequence
    pub read_cfg_reg2: Option<NorStorageCmd>,
    /// Write Config2 Register Sequence
    pub write_cfg_reg2: Option<NorStorageCmd>,
    /// Read Config3 Register Sequence
    pub read_cfg_reg3: Option<NorStorageCmd>,
    /// Write Config3 Register Sequence
    pub write_cfg_reg3: Option<NorStorageCmd>,
    /// chip erase
    pub chip_erase: Option<NorStorageCmd>,
}

/// NAND Storage Command Sequence to be passed by NAND based storage device drivers
pub struct NandStorageCmdSequence {
    // TODO
}

/// Config Storage  Command sequences
pub trait ConfigureCmdSeq {
    /// Configure the storage command sequences
    fn configure_cmd_seq(&self, cmd_seq: &NorStorageCmdSeq);
}

/// Blocking NOR Storage Driver
pub trait BlockingNorStorageDriver: BlockingNorFlash + BlockingReadNorFlash + ConfigureCmdSeq {
    /// Lock the storage
    fn lock(&self) -> Result<(), Self::Error>;
    /// Unlock the storage
    fn unlock(&self) -> Result<(), Self::Error>;
    /// Read the JEDEC ID
    fn read_jedec_id(&self) -> Result<[u8; 3], Self::Error>;
    /// Power down the storage
    fn power_down(&self) -> Result<(), Self::Error>;
    /// Power up the storage
    fn power_up(&self) -> Result<(), Self::Error>;
    /// Write Enable
    fn write_enable(&mut self) -> Result<(), Self::Error>;
    /// Write Disable
    fn write_disable(&mut self) -> Result<(), Self::Error>;
    /// Read Status Register
    fn read_status_reg(&mut self) -> Result<[u8; 4], Self::Error>;
    /// Chip Erase
    fn chip_erase(&mut self) -> Result<(), Self::Error>;
}

/// Async NOR Storage Driver
pub trait AsyncNorStorageDriver: AsyncNorFlash + AsyncReadNorFlash + ConfigureCmdSeq {
    /// Lock the storage
    async fn lock(&self) -> Result<(), Self::Error>;
    /// Unlock the storage
    async fn unlock(&self) -> Result<(), Self::Error>;
    /// Read the JEDEC ID
    async fn read_jedec_id(&self) -> Result<[u8; 3], Self::Error>;
    /// Power down the storage
    async fn power_down(&self) -> Result<(), Self::Error>;
    /// Power up the storage
    async fn power_up(&self) -> Result<(), Self::Error>;
    /// Write Enable
    async fn write_enable(&self) -> Result<(), Self::Error>;
    /// Write Disable
    async fn write_disable(&self) -> Result<(), Self::Error>;
    /// Read Status Register
    async fn read_status_reg(&mut self) -> Result<[u8; 4], Self::Error>;
    /// Chip Erase
    async fn chip_erase(&mut self) -> Result<(), Self::Error>;
}

/// Blocking NAND storage driver
pub trait BlockingNandStorageDriver {}

/// Async NAND storage driver
pub trait AsyncNandStorageDriver {}

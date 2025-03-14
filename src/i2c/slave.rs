//! Implements I2C function support over flexcomm + gpios

use core::future::poll_fn;
use core::marker::PhantomData;
use core::task::Poll;

use embassy_hal_internal::{into_ref, Peripheral};

use super::{
    Async, Blocking, Info, Instance, InterruptHandler, Mode, Result, SclPin, SdaPin, SlaveDma, TransferError,
    I2C_WAKERS,
};
use crate::interrupt::typelevel::Interrupt;
use crate::pac::i2c0::stat::Slvstate;
use crate::{dma, interrupt};

/// I2C address type
#[derive(Copy, Clone, Debug)]
pub struct Address(u8);

impl Address {
    /// Construct an address type
    #[must_use]
    pub const fn new(addr: u8) -> Option<Self> {
        match addr {
            0x08..=0x77 => Some(Self(addr)),
            _ => None,
        }
    }

    /// interpret address as a read command
    #[must_use]
    pub fn read(&self) -> u8 {
        (self.0 << 1) | 1
    }

    /// interpret address as a write command
    #[must_use]
    pub fn write(&self) -> u8 {
        self.0 << 1
    }
}

impl From<Address> for u8 {
    fn from(value: Address) -> Self {
        value.0
    }
}

/// Command from master
pub enum Command {
    /// I2C probe with no data
    Probe,

    /// I2C Read
    Read,

    /// I2C Write
    Write,
}

/// Result of response functions
pub enum Response {
    /// I2C transaction complete with this amount of bytes
    Complete(usize),

    /// I2C transaction pending wutg this amount of bytes completed so far
    Pending(usize),
}

/// use `FCn` as I2C Slave controller
pub struct I2cSlave<'a, M: Mode> {
    info: Info,
    _phantom: PhantomData<M>,
    dma_ch: Option<dma::channel::Channel<'a>>,
}

impl<'a, M: Mode> I2cSlave<'a, M> {
    /// use flexcomm fc with Pins scl, sda as an I2C Master bus, configuring to speed and pull
    fn new_inner<T: Instance>(
        _bus: impl Peripheral<P = T> + 'a,
        scl: impl Peripheral<P = impl SclPin<T>> + 'a,
        sda: impl Peripheral<P = impl SdaPin<T>> + 'a,
        // TODO - integrate clock APIs to allow dynamic freq selection | clock: crate::flexcomm::Clock,
        address: Address,
        dma_ch: Option<dma::channel::Channel<'a>>,
    ) -> Result<Self> {
        into_ref!(_bus);
        into_ref!(scl);
        into_ref!(sda);

        sda.as_sda();
        scl.as_scl();

        // this check should be redundant with T::set_mode()? above
        let info = T::info();
        let i2c = info.regs;

        // rates taken assuming SFRO:
        //
        //  7 => 403.3 kHz
        //  9 => 322.6 kHz
        // 12 => 247.8 kHz
        // 16 => 198.2 kHz
        // 18 => 166.6 Khz
        // 22 => 142.6 kHz
        // 30 => 100.0 kHz
        // UM10204 pg.44 rev7
        // tSU;DAT >= 250ns -> < 250MHz
        i2c.clkdiv().write(|w|
            // SAFETY: only unsafe due to .bits usage
            unsafe { w.divval().bits(0) });

        // address 0 match = addr, per UM11147 24.3.2.1
        i2c.slvadr(0).modify(|_, w|
            // note: shift is omitted as performed via w.slvadr() 
            // SAFETY: unsafe only required due to use of unnamed "bits" field
            unsafe {w.slvadr().bits(address.0)}.sadisable().enabled());

        // SLVEN = 1, per UM11147 24.3.2.1
        i2c.cfg().write(|w| w.slven().enabled());

        Ok(Self {
            info,
            _phantom: PhantomData,
            dma_ch,
        })
    }
}

impl<'a> I2cSlave<'a, Blocking> {
    /// use flexcomm fc with Pins scl, sda as an I2C Master bus, configuring to speed and pull
    pub fn new_blocking<T: Instance>(
        _bus: impl Peripheral<P = T> + 'a,
        scl: impl Peripheral<P = impl SclPin<T>> + 'a,
        sda: impl Peripheral<P = impl SdaPin<T>> + 'a,
        // TODO - integrate clock APIs to allow dynamic freq selection | clock: crate::flexcomm::Clock,
        address: Address,
    ) -> Result<Self> {
        // TODO - clock integration
        let clock = crate::flexcomm::Clock::Sfro;
        T::enable(clock);
        T::into_i2c();

        Self::new_inner::<T>(_bus, scl, sda, address, None)
    }

    fn poll(&self) -> Result<()> {
        let i2c = self.info.regs;

        while i2c.stat().read().slvpending().is_in_progress() {}

        Ok(())
    }

    fn block_until_addressed(&self) -> Result<()> {
        self.poll()?;

        let i2c = self.info.regs;
        if !i2c.stat().read().slvstate().is_slave_address() {
            return Err(TransferError::AddressNack.into());
        }

        i2c.slvctl().write(|w| w.slvcontinue().continue_());
        Ok(())
    }
}

impl<'a> I2cSlave<'a, Async> {
    /// use flexcomm fc with Pins scl, sda as an I2C Master bus, configuring to speed and pull
    pub fn new_async<T: Instance>(
        _bus: impl Peripheral<P = T> + 'a,
        scl: impl Peripheral<P = impl SclPin<T>> + 'a,
        sda: impl Peripheral<P = impl SdaPin<T>> + 'a,
        _irq: impl interrupt::typelevel::Binding<T::Interrupt, InterruptHandler<T>> + 'a,
        // TODO - integrate clock APIs to allow dynamic freq selection | clock: crate::flexcomm::Clock,
        address: Address,
        dma_ch: impl Peripheral<P = impl SlaveDma<T>> + 'a,
    ) -> Result<Self> {
        // TODO - clock integration
        let clock = crate::flexcomm::Clock::Sfro;
        T::enable(clock);
        T::into_i2c();

        let ch = dma::Dma::reserve_channel(dma_ch);
        let this = Self::new_inner::<T>(_bus, scl, sda, address, Some(ch))?;

        T::Interrupt::unpend();
        unsafe { T::Interrupt::enable() };

        Ok(this)
    }
}

impl I2cSlave<'_, Blocking> {
    /// Listen for commands from the I2C Master.
    pub fn listen(&self) -> Result<Command> {
        let i2c = self.info.regs;

        self.block_until_addressed()?;

        //Block until we know it is read or write
        self.poll()?;

        // We are already deselected, so it must be an 0 byte write transaction
        if i2c.stat().read().slvdesel().is_deselected() {
            // Clear the deselected bit
            i2c.stat().write(|w| w.slvdesel().deselected());
            return Ok(Command::Probe);
        }

        let state = i2c.stat().read().slvstate().variant();
        match state {
            Some(Slvstate::SlaveReceive) => Ok(Command::Write),
            Some(Slvstate::SlaveTransmit) => Ok(Command::Read),
            _ => Err(TransferError::OtherBusError.into()),
        }
    }

    /// Respond to write command from  master
    pub fn respond_to_write(&self, buf: &mut [u8]) -> Result<Response> {
        let i2c = self.info.regs;
        let mut xfer_count: usize = 0;

        for b in buf {
            //poll until something happens
            self.poll()?;

            let stat = i2c.stat().read();
            // if master send stop, we are done
            if stat.slvdesel().is_deselected() {
                break;
            }
            // if master send a restart, we are done
            if stat.slvstate().is_slave_address() {
                break;
            }

            if !stat.slvstate().is_slave_receive() {
                return Err(TransferError::ReadFail.into());
            }

            // Now we can safely read the next byte
            *b = i2c.slvdat().read().data().bits();
            i2c.slvctl().write(|w| w.slvcontinue().continue_());
            xfer_count += 1;
        }

        let stat = i2c.stat().read();
        if stat.slvdesel().is_deselected() {
            // Clear the deselect bit
            i2c.stat().write(|w| w.slvdesel().deselected());
            return Ok(Response::Complete(xfer_count));
        } else if stat.slvstate().is_slave_address() {
            // Handle restart
            return Ok(Response::Complete(xfer_count));
        } else if stat.slvstate().is_slave_receive() {
            // Master still wants to send more data, transaction incomplete
            return Ok(Response::Pending(xfer_count));
        }

        // We should not get here
        Err(TransferError::ReadFail.into())
    }

    /// Respond to read command from  master
    pub fn respond_to_read(&self, buf: &[u8]) -> Result<Response> {
        let i2c = self.info.regs;
        let mut xfer_count: usize = 0;

        for b in buf {
            // Block until something happens
            self.poll()?;

            let stat = i2c.stat().read();
            // if master send nack or stop, we are done
            if stat.slvdesel().is_deselected() {
                break;
            }
            // if master send restart, we are done
            if stat.slvstate().is_slave_address() {
                break;
            }

            // Verify that we are ready for write
            if !stat.slvstate().is_slave_transmit() {
                return Err(TransferError::WriteFail.into());
            }

            i2c.slvdat().write(|w|
                // SAFETY: unsafe only here due to use of bits()
                unsafe{w.data().bits(*b)});
            i2c.slvctl().write(|w| w.slvcontinue().continue_());
            xfer_count += 1;
        }

        let stat = i2c.stat().read();
        if stat.slvdesel().is_deselected() {
            // clear the deselect bit
            i2c.stat().write(|w| w.slvdesel().deselected());
            return Ok(Response::Complete(xfer_count));
        } else if stat.slvstate().is_slave_address() {
            // Handle restart after read
            return Ok(Response::Complete(xfer_count));
        } else if stat.slvstate().is_slave_transmit() {
            // Master is still expecting data, transaction incomplete
            return Ok(Response::Pending(xfer_count));
        }

        // We should not get here
        Err(TransferError::WriteFail.into())
    }
}

impl I2cSlave<'_, Async> {
    /// Listen for commands from the I2C Master asynchronously
    pub async fn listen(&mut self) -> Result<Command> {
        let i2c = self.info.regs;

        // Disable DMA
        i2c.slvctl().write(|w| w.slvdma().disabled());

        // Check whether we already have a matched address and just waiting
        // for software ack/nack
        if !i2c.stat().read().slvpending().is_pending() {
            self.poll_sw_action().await;
        }

        if i2c.stat().read().slvstate().is_slave_address() {
            i2c.slvctl().write(|w| w.slvcontinue().continue_());
        } else {
            // If we are not addressed here, then we have issues.
            return Err(TransferError::OtherBusError.into());
        }

        // Poll for HW to transitioning from addressed to receive/transmit
        self.poll_sw_action().await;

        // We are deselected, so it must be an 0 byte write transaction
        if i2c.stat().read().slvdesel().is_deselected() {
            // Clear the deselected bit
            i2c.stat().write(|w| w.slvdesel().deselected());
            return Ok(Command::Probe);
        }

        let state = i2c.stat().read().slvstate().variant();
        match state {
            Some(Slvstate::SlaveReceive) => Ok(Command::Write),
            Some(Slvstate::SlaveTransmit) => Ok(Command::Read),
            _ => Err(TransferError::OtherBusError.into()),
        }
    }

    /// Respond to write command from master
    pub async fn respond_to_write(&mut self, buf: &mut [u8]) -> Result<Response> {
        let i2c = self.info.regs;
        let buf_len = buf.len();

        // Verify that we are ready for write
        let stat = i2c.stat().read();
        if !stat.slvstate().is_slave_receive() {
            // 0 byte write
            if stat.slvdesel().is_deselected() {
                return Ok(Response::Complete(0));
            }
            return Err(TransferError::ReadFail.into());
        }

        // Enable DMA
        i2c.slvctl().write(|w| w.slvdma().enabled());

        // Enable interrupt
        i2c.intenset()
            .write(|w| w.slvpendingen().enabled().slvdeselen().enabled());

        let options = dma::transfer::TransferOptions::default();
        // Keep a reference to Transfer so it does not get dropped and aborted the DMA transfer
        let _transfer =
            self.dma_ch
                .as_ref()
                .unwrap()
                .read_from_peripheral(i2c.slvdat().as_ptr() as *mut u8, buf, options);

        poll_fn(|cx| {
            let i2c = self.info.regs;
            let dma = self.dma_ch.as_ref().unwrap();

            I2C_WAKERS[self.info.index].register(cx.waker());
            dma.get_waker().register(cx.waker());

            let stat = i2c.stat().read();
            // Did master send a stop?
            if stat.slvdesel().is_deselected() {
                return Poll::Ready(());
            }
            // Does SW need to intervene?
            if stat.slvpending().is_pending() {
                return Poll::Ready(());
            }
            // Did we complete the DMA transfer and does the master still have more data for us?
            if !dma.is_active() && stat.slvstate().is_slave_receive() {
                return Poll::Ready(());
            }

            Poll::Pending
        })
        .await;

        // Complete DMA transaction and get transfer count
        let xfer_count = self.abort_dma(buf_len);
        let stat = i2c.stat().read();
        // We got a stop from master, either way this transaction is
        // completed
        if stat.slvdesel().is_deselected() {
            // Clear the deselected bit
            i2c.stat().write(|w| w.slvdesel().deselected());

            return Ok(Response::Complete(xfer_count));
        } else if stat.slvstate().is_slave_address() {
            // We are addressed again, so this must be a restart
            return Ok(Response::Complete(xfer_count));
        } else if stat.slvstate().is_slave_receive() {
            // That was a partial transaction, the master want to send more
            // data
            return Ok(Response::Pending(xfer_count));
        }

        Err(TransferError::ReadFail.into())
    }

    /// Respond to read command from master
    /// User must provide enough data to complete the transaction or else
    ///    we will get stuck in this function
    pub async fn respond_to_read(&mut self, buf: &[u8]) -> Result<Response> {
        let i2c = self.info.regs;

        // Verify that we are ready for transmit
        if !i2c.stat().read().slvstate().is_slave_transmit() {
            return Err(TransferError::WriteFail.into());
        }

        // Enable DMA
        i2c.slvctl().write(|w| w.slvdma().enabled());

        // Enable interrupts
        i2c.intenset()
            .write(|w| w.slvpendingen().enabled().slvdeselen().enabled());

        let options = dma::transfer::TransferOptions::default();
        // Keep a reference to Transfer so it does not get dropped and aborted the DMA transfer
        let _transfer =
            self.dma_ch
                .as_ref()
                .unwrap()
                .write_to_peripheral(buf, i2c.slvdat().as_ptr() as *mut u8, options);

        poll_fn(|cx| {
            let i2c = self.info.regs;
            let dma = self.dma_ch.as_ref().unwrap();

            I2C_WAKERS[self.info.index].register(cx.waker());
            dma.get_waker().register(cx.waker());

            let stat = i2c.stat().read();
            // Master sent a stop or nack
            if stat.slvdesel().is_deselected() {
                return Poll::Ready(());
            }
            // We need SW intervention
            if stat.slvpending().is_pending() {
                return Poll::Ready(());
            }

            Poll::Pending
        })
        .await;

        // Complete DMA transaction and get transfer count
        let xfer_count = self.abort_dma(buf.len());
        let stat = i2c.stat().read();

        // we got a nack or a stop from master, either way this transaction is
        // completed
        if stat.slvdesel().is_deselected() {
            // clear the deselect bit
            i2c.stat().write(|w| w.slvdesel().deselected());
            return Ok(Response::Complete(xfer_count));
        } else if stat.slvpending().is_pending() || stat.slvstate().is_slave_address() {
            // Handle restart after read as well as the cases where
            // slave deselected is not set in response to a master nack
            // then the next transaction starts the slave state goes into
            // pending + addressed.
            return Ok(Response::Complete(xfer_count));
        }

        // We should not get here
        Err(TransferError::WriteFail.into())
    }

    async fn poll_sw_action(&self) {
        let i2c = self.info.regs;

        i2c.intenset()
            .write(|w| w.slvpendingen().enabled().slvdeselen().enabled());

        poll_fn(|cx: &mut core::task::Context<'_>| {
            I2C_WAKERS[self.info.index].register(cx.waker());

            let stat = i2c.stat().read();
            if stat.slvdesel().is_deselected() {
                return Poll::Ready(());
            }
            if stat.slvpending().is_pending() {
                return Poll::Ready(());
            }

            Poll::Pending
        })
        .await;
    }

    /// Complete DMA and return bytes transfer
    fn abort_dma(&self, xfer_size: usize) -> usize {
        // abort DMA if DMA is not compelted
        let dma = self.dma_ch.as_ref().unwrap();
        let remain_xfer_count = dma.get_xfer_count();
        let mut xfer_count = xfer_size;
        if dma.is_active() && remain_xfer_count != 0x3FF {
            xfer_count -= remain_xfer_count as usize + 1;
            dma.abort();
        }

        xfer_count
    }
}

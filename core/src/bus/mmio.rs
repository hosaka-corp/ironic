
use crate::bus::*;
use crate::bus::prim::*;
use crate::bus::task::*;

/// Interface used by the bus to perform some access on an I/O device.
pub trait MmioDevice {
    /// Width of accesses supported on this device.
    type Width;

    /// Handle a read, returning some result.
    fn read(&mut self, off: usize) -> BusPacket;
    /// Handle a write, optionally returning a task for the bus.
    fn write(&mut self, off: usize, val: Self::Width) -> Option<BusTask>;
}

impl Bus {
    /// Dispatch a physical read access to some memory-mapped I/O device.
    pub fn do_mmio_read(&mut self, dev: IoDevice, off: usize, width: BusWidth) -> BusPacket {
        use IoDevice::*;
        match (width, dev) {
            (BusWidth::W, Nand)  => self.nand.read(off),
            (BusWidth::W, Aes)   => self.aes.read(off),
            (BusWidth::W, Sha)   => self.sha.read(off),
            (BusWidth::W, Ehci)  => self.ehci.read(off),
            (BusWidth::W, Ohci0) => self.ohci0.read(off),
            (BusWidth::W, Ohci1) => self.ohci1.read(off),
            (BusWidth::W, Sdhc0) => self.sd0.read(off),
            (BusWidth::W, Sdhc1) => self.sd1.read(off),

            (BusWidth::W, Hlwd)  => self.hlwd.read(off),
            (BusWidth::W, Ahb)   => self.hlwd.ahb.read(off),
            (BusWidth::W, Di)    => self.hlwd.di.read(off),
            (BusWidth::W, Exi)   => self.hlwd.exi.read(off),
            (BusWidth::H, Mi)    => self.hlwd.mi.read(off),
            (BusWidth::H, Ddr)   => self.hlwd.ddr.read(off),
            _ => panic!("Unsupported read {:?} for {:?} at {:x}", width, dev, off),
        }
    }

    /// Dispatch a physical write access to some memory-mapped I/O device.
    pub fn do_mmio_write(&mut self, dev: IoDevice, off: usize, msg: BusPacket) {
        use IoDevice::*;
        use BusPacket::*;
        let task = match (msg, dev) {
            (Word(val), Nand)  => self.nand.write(off, val),
            (Word(val), Aes)   => self.aes.write(off, val),
            (Word(val), Sha)   => self.sha.write(off, val),
            (Word(val), Ehci)  => self.ehci.write(off, val),
            (Word(val), Ohci0) => self.ohci0.write(off, val),
            (Word(val), Ohci1) => self.ohci1.write(off, val),
            (Word(val), Sdhc0) => self.sd0.write(off, val),
            (Word(val), Sdhc1) => self.sd1.write(off, val),


            (Word(val), Hlwd)  => self.hlwd.write(off, val),
            (Word(val), Ahb)   => self.hlwd.ahb.write(off, val),
            (Word(val), Exi)   => self.hlwd.exi.write(off, val),
            (Word(val), Di)    => self.hlwd.di.write(off, val),
            (Half(val), Mi)    => self.hlwd.mi.write(off, val),
            (Half(val), Ddr)   => self.hlwd.ddr.write(off, val),

            _ => panic!("Unsupported write {:?} for {:?} at {:x}", msg, dev, off),
        };

        // If the device returned some task, schedule it
        if task.is_some() {
            let t = task.unwrap();
            let c = match t {
                BusTask::Nand(_) => 0,
                BusTask::Aes(_) => 0,
                BusTask::Sha(_) => 0,

                BusTask::Mi{..} => 0,
                BusTask::SetRomDisabled(_) => 0,
                BusTask::SetMirrorEnabled(_) => 0,
            };
            self.tasks.push(Task { kind: t, target_cycle: self.cycle + c });
        }
    }
}


impl Bus {
    /// Emulate a slice of work on the system bus.
    pub fn step(&mut self, cpu_cycle: usize) {
        self.handle_step_hlwd(cpu_cycle);
        if !self.tasks.is_empty() {
            self.drain_tasks();
        }
        self.cycle += 1;
    }

    /// Dispatch all of the pending tasks on the Bus.
    fn drain_tasks(&mut self) {
        let mut idx = 0;
        while idx != self.tasks.len() {
            if self.tasks[idx].target_cycle <= self.cycle {
                let task = self.tasks.remove(idx);
                match task.kind {
                    BusTask::Nand(x) => self.handle_task_nand(x),
                    BusTask::Aes(x) => self.handle_task_aes(x),
                    BusTask::Sha(x) => self.handle_task_sha(x),
                    BusTask::Mi{kind, data} => self.handle_task_mi(kind, data),
                    BusTask::SetRomDisabled(x) => self.rom_disabled = x,
                    BusTask::SetMirrorEnabled(x) => self.mirror_enabled = x,
                }
            } else {
                idx += 1;
            }
        }
    }
}


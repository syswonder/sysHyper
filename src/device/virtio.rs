use alloc::collections::{BTreeMap, LinkedList};
use spin::Mutex;

use crate::{
    cell::this_cell_id,
    control::{send_event, suspend_self},
    error::HvResult,
    hypercall::SGI_VIRTIO_REQ_ID,
    memory::MMIOAccess,
    percpu::this_cpu_id,
};

use super::gicv3::inject_irq;
/// When non root's virtio driver communicates with device, the message will be stored this req list and passed to root cell.
pub static TRAMPOLINE_REQ_LIST: Mutex<LinkedList<VirtioReq>> = Mutex::new(LinkedList::new());
/// cpu_id: value(irq_id || returned value)
pub static VIRTIO_RESULT_MAP: Mutex<BTreeMap<u64, u64>> = Mutex::new(BTreeMap::new());
const QUEUE_NOTIFY: usize = 0x50;
pub struct VirtioReq {
    pub src_cell: u32,
    pub src_cpu: u64,
    pub is_cfg: bool,
    // mmio.address is ipa
    pub mmio: MMIOAccess,
}

impl VirtioReq {
    fn new(src_cell: u32, src_cpu: u64, is_cfg: bool, mmio: MMIOAccess) -> Self {
        Self {
            src_cell,
            src_cpu,
            is_cfg,
            mmio,
        }
    }
}

/// non root cell's virtio request handler
pub fn mmio_virtio_handler(mmio: &mut MMIOAccess, base: u64) -> HvResult {
    debug!("mmio virtio handler");
    let is_cfg = mmio.address != QUEUE_NOTIFY;
    if !is_cfg {
        info!("notify !!!, cpu id is {}", this_cpu_id());
    }
    mmio.address += base as usize;
    let mut req_list = TRAMPOLINE_REQ_LIST.lock();
    req_list.push_back(VirtioReq::new(
        this_cell_id(),
        this_cpu_id(),
        is_cfg,
        mmio.clone(),
    ));
    drop(req_list);
    send_event(0, SGI_VIRTIO_REQ_ID);
    // if it is cfg request, current cpu should be blocked until gets the result
    if is_cfg {
        // block current cpu
        suspend_self();
        // current cpu waked up
        if !mmio.is_write {
            let map = VIRTIO_RESULT_MAP.lock();
            mmio.value = *map.get(&this_cpu_id()).unwrap();
            // Attention: If map is a list, 无论mmio是否为is_write都需要把值取出来
            debug!("non root receives value: {:#x?}", mmio.value);
        }
    }
    info!("non root returns");
    Ok(())
}

/// When virtio req type is notify, root cell will send sgi to non root, \
/// and non root will call this function.
pub fn handle_virtio_result() {
    info!("notify resolved");
    let map = VIRTIO_RESULT_MAP.lock();
    let irq_id = map.get(&this_cpu_id()).unwrap();
    inject_irq(*irq_id as _, false);
}
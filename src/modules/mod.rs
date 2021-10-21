use skyline::nro::{NroInfo, add_hook};
use smash::lua2cpp::*;
use smash::app::BattleObjectModuleAccessor;
use std::sync::Once;

use crate::debugln;

// pub mod anim;
pub mod buffer;
pub mod meter;
pub mod param;
pub mod var;

// pub use anim::*;
pub use buffer::*;
pub use meter::*;
pub use param::*;
pub use var::*;

const HDR_BOMA_MAGIC: u64 = 0x4844524d41474943;

pub fn is_hdr_boma(boma: *mut BattleObjectModuleAccessor) -> bool {
    if boma.is_null() {
        false
    } else {
        unsafe {
            let vtable = *(boma as *const *const u64);
            *vtable.offset(-2) == HDR_BOMA_MAGIC
        }
    }
}

// Offsets from the end of the vtable (each member is expected to be an 8-byte value)
const                OG_VTABLE_OFFSET: isize = 0;
pub(crate) const    VAR_MODULE_OFFSET: isize = 1;
pub(crate) const  PARAM_MODULE_OFFSET: isize = 2;
pub(crate) const  METER_MODULE_OFFSET: isize = 3;
pub(crate) const BUFFER_MODULE_OFFSET: isize = 4;

static INIT: Once = Once::new();

unsafe fn create_vtable_layout(count: isize) -> std::alloc::Layout {
    std::alloc::Layout::from_size_align(((count + 2 + BUFFER_MODULE_OFFSET + 1) * 8) as usize, 0x10).expect("Unable to create vtable layout.")
}

unsafe fn common_dtor(boma: *mut BattleObjectModuleAccessor) {
    let mut vtable = *(boma as *const *const u64);
    if *vtable.offset(-2) == HDR_BOMA_MAGIC {
        let entry_count = *vtable.offset(-1) as isize;
        let layout = create_vtable_layout(entry_count);
        let og_vtable = *vtable.offset(entry_count + OG_VTABLE_OFFSET) as *const u64;
        drop(Box::from_raw(*vtable.offset(entry_count + VAR_MODULE_OFFSET) as *mut VarModule));
        drop(Box::from_raw(*vtable.offset(entry_count + PARAM_MODULE_OFFSET) as *mut ParamModule));
        drop(Box::from_raw(*vtable.offset(entry_count + METER_MODULE_OFFSET) as *mut MeterModule));
        drop(Box::from_raw(*vtable.offset(entry_count + BUFFER_MODULE_OFFSET) as *mut BufferModule));
        vtable = vtable.offset(-2);
        std::alloc::dealloc(vtable as *mut u8, layout);
        *(boma as *mut *const u64) = og_vtable;
    }
}

#[allow(non_snake_case)]
unsafe extern "C" fn BattleObjectModuleAccessor_destructor(boma: *mut BattleObjectModuleAccessor) {
    common_dtor(boma);
    let callable: extern "C" fn(*mut BattleObjectModuleAccessor) = std::mem::transmute((*(boma as *const *const u64)).offset(1));
    callable(boma)
}

#[allow(non_snake_case)]
unsafe extern "C" fn BattleObjectModuleAccessor_delete_destructor(boma: *mut BattleObjectModuleAccessor) {
    common_dtor(boma);
    let callable: extern "C" fn(*mut BattleObjectModuleAccessor) = std::mem::transmute((*(boma as *const *const u64)).offset(2));
    callable(boma)
}

// changes when a new one is found, should not be very many
static mut BATTLE_OBJECT_MODULE_ACCESSOR_DTOR: usize = 0;
#[skyline::hook(replace = BATTLE_OBJECT_MODULE_ACCESSOR_DTOR, inline)]
unsafe fn destructor_hook(ctx: &skyline::hooks::InlineCtx) {
    common_dtor(*ctx.registers[0].x.as_ref() as *mut BattleObjectModuleAccessor);
}


unsafe fn hook_destructors(vtable: *const u64) {
    static mut DESTRUCTOR_VECTOR: parking_lot::Mutex<Vec<u64>> = parking_lot::Mutex::new(Vec::new());
    let mut destructors = DESTRUCTOR_VECTOR.lock();
    if !destructors.contains(&*vtable.offset(1)) {
        BATTLE_OBJECT_MODULE_ACCESSOR_DTOR = *vtable.offset(1) as usize;
        skyline::install_hook!(destructor_hook);
        destructors.push(*vtable.offset(1));
    }
    if !destructors.contains(&*vtable.offset(2)) {
        BATTLE_OBJECT_MODULE_ACCESSOR_DTOR = *vtable.offset(2) as usize;
        skyline::install_hook!(destructor_hook);
        destructors.push(*vtable.offset(2));
    }
}

unsafe fn recreate_vtable(boma: *mut BattleObjectModuleAccessor, category: i32, kind: i32) {
    let vtable = *(boma as *const *const u64);
    if *vtable.offset(-2) != HDR_BOMA_MAGIC {
        let mut entry_count = 0isize;
        loop {
            if *vtable.offset(entry_count) == 0 { break; }
            entry_count += 1;
        }
        hook_destructors(vtable);
        let layout = create_vtable_layout(entry_count);
        let mut new_vtable = std::alloc::alloc(layout) as *mut u64;
        new_vtable = new_vtable.offset(2);
        std::ptr::copy_nonoverlapping(vtable, new_vtable, entry_count as usize);
        *new_vtable.offset(-2) = HDR_BOMA_MAGIC;
        *new_vtable.offset(-1) = entry_count as u64;
        *new_vtable.offset(entry_count + OG_VTABLE_OFFSET) = vtable as u64;
        *new_vtable.offset(entry_count + VAR_MODULE_OFFSET) = Box::into_raw(Box::new(VarModule::new())) as u64;
        *new_vtable.offset(entry_count + PARAM_MODULE_OFFSET) = Box::into_raw(Box::new(ParamModule::new(category, kind))) as u64;
        *new_vtable.offset(entry_count + METER_MODULE_OFFSET) = Box::into_raw(Box::new(MeterModule::new(30, 10))) as u64;
        *new_vtable.offset(entry_count + BUFFER_MODULE_OFFSET) = Box::into_raw(Box::new(BufferModule::new(boma))) as u64;
        *new_vtable.offset(1) = std::mem::transmute(BattleObjectModuleAccessor_destructor as *const extern "C" fn()); // these don't get called sadge
        *new_vtable.offset(2) = std::mem::transmute(BattleObjectModuleAccessor_delete_destructor as *const extern "C" fn()); // these don't get called sadge
        *(boma as *mut *const u64) = new_vtable;
        // anim::add_to_module_accessor(boma, kind);
    }
    else {
        debugln!("[HDR] BattleObjectModuleAccessor has already been converted -- skipping.");
    }
}

#[smashline::fighter_init]
fn L2CFighterCommon_sys_line_system_init(fighter: &mut L2CFighterCommon) {
    unsafe {
        let category = smash::app::sv_system::battle_object_category(fighter.lua_state_agent);
        let kind = smash::app::sv_system::battle_object_kind(fighter.lua_state_agent);
        recreate_vtable(fighter.module_accessor, category as i32, kind);
    }
}

pub fn init() {
    unsafe {
        INIT.call_once(|| {
            smashline::install_agent_init_callbacks!(
                L2CFighterCommon_sys_line_system_init
            );
            buffer::init();
        });
    }
}
use smash::app::BattleObjectModuleAccessor;
use lazy_static::lazy_static;
use std::sync::atomic::Ordering;
use parking_lot::Mutex;
use std::collections::HashMap;

lazy_static! {
    static ref NEW_ANIMS: Mutex<HashMap<String, Vec<(u32, u64, usize)>>> = Mutex::new(HashMap::new());
}

unsafe fn add_animation(boma: *mut BattleObjectModuleAccessor, resource_id: u64, table1_idx: u32) -> bool {
    let motion_module = *(((boma as u64) + 0x88) as *const u64);
    let motion_list = *((motion_module + 0x148) as *const u64);
    let animation_map = *((motion_list + 0x70) as *const u64);
    let func = *((*(animation_map as *const u64) + 0x10) as *const u64);
    let callable: extern "C" fn(u64, u64, *const u32, bool) -> bool = std::mem::transmute(func);
    callable(animation_map, resource_id, &table1_idx, false)
}

unsafe fn memcpy_file_data(data: &Vec<u8>) -> *mut u8 {
    let layout = std::alloc::Layout::from_size_align(data.len(), 1).unwrap();
    let ret_ptr = std::alloc::alloc(layout);
    std::ptr::copy_nonoverlapping(data.as_ptr(), ret_ptr, data.len());
    ret_ptr
}

unsafe fn free_file_data(ptr: *mut u8, sz: usize) {
    let layout = std::alloc::Layout::from_size_align(sz, 1).unwrap();
    std::alloc::dealloc(ptr, layout);
}

pub(crate) fn handle_nuanmb_load(path: String, data: Vec<u8>) {
    unsafe {
        let hash = smash::phx::Hash40::new(&path).hash;
        let added_files = crate::fs::ADDED_FILES.lock();
        if let Some((table1_idx, file_size)) = added_files.get(&hash) {
            let instance = crate::arc_runtime::LoadedTables::get_instance();
            let table1 = crate::arc_runtime::LoadedTables::get_table1();
            let table2 = crate::arc_runtime::LoadedTables::get_table2_mut();
            skyline::nn::os::LockMutex(instance.mutex);
            assert!(table1[*table1_idx as usize].in_table_2 != 0, "Custom file loaded not in table 2!");
            let table2_idx = table1[*table1_idx as usize].table2_index;
            let table2_entry = table2.get_mut(table2_idx as usize).expect("Table1Entry held invalid Table2 index");
            let ref_cnt = table2_entry.ref_count.load(Ordering::Acquire);
            assert!(table2_entry.data.is_null(), "Reloaded custom file while previous is still loaded.");
            table2_entry.data = memcpy_file_data(&data);
            table2_entry.ref_count.fetch_add(1, Ordering::AcqRel);
            table2_entry.state = 3;
            table2_entry.is_used = true;
            skyline::nn::os::UnlockMutex(instance.mutex);
            // load table1 idx and resource id into hashmap
            let tokens: Vec<String> = path.split('/').map(|x| String::from(x)).collect();
            let agent = tokens.get(2).expect("Invalid unique fighter path.");
            let resource = tokens.last().unwrap();
            let resource_id = smash::phx::Hash40::new(resource).hash;
            let mut anim_map = NEW_ANIMS.lock();
            if let Some(agent_anims) = anim_map.get_mut(agent) {
                agent_anims.push((*table1_idx, resource_id, *file_size as usize));
            } else {
                anim_map.insert(agent.clone(), vec![(*table1_idx, resource_id, *file_size as usize)]);
            }
        }
    }
}

pub(crate) fn handle_nuanmb_unload(info: &skyline::nro::NroInfo) {
    unsafe {
        let mut new_anims = NEW_ANIMS.lock();
        if let Some(anim_list) = new_anims.remove(&String::from(info.name)) {
            let instance = crate::arc_runtime::LoadedTables::get_instance();
            let table1 = crate::arc_runtime::LoadedTables::get_table1();
            let table2 = crate::arc_runtime::LoadedTables::get_table2_mut();
            skyline::nn::os::LockMutex(instance.mutex);
            for (table1_idx, resource_id, file_size) in anim_list.iter() {
                assert!(table1[*table1_idx as usize].in_table_2 != 0, "Custom filed loaded not in table 2!");
                let table2_idx = table1[*table1_idx as usize].table2_index;
                let table2_entry = table2.get_mut(table2_idx as usize).expect("Table1Entry held invalid Table2 index");
                let ref_cnt = table2_entry.ref_count.load(Ordering::Acquire);
                assert!(ref_cnt != 0, "Trying to unload previously unloaded data.");
                assert!(ref_cnt == 1, "Trying to unload data still in use");
                table2_entry.ref_count.fetch_sub(1, Ordering::AcqRel);
                free_file_data(table2_entry.data as *mut u8, *file_size);
                table2_entry.data = 0 as _;
                table2_entry.is_used = false;
                table2_entry.state = 0;
            }
            skyline::nn::os::UnlockMutex(instance.mutex);
        }
    }
}

pub(crate) fn add_to_module_accessor(boma: *mut BattleObjectModuleAccessor, kind: i32) {
    let new_anims = NEW_ANIMS.lock();
    for (agent, anim_list) in new_anims.iter() {
        if crate::utils::agent_to_agent_kind(agent) == kind {
            for (table1_idx, res_id, file_size) in anim_list.iter() {
                unsafe {
                    assert!(add_animation(boma, *res_id, *table1_idx), "Failed to add animation to boma.");
                }
            }
        }
    }
}
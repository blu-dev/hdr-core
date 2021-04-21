// shamelessly stolen from Raytwo's Arcropolis :))))))))

use std::sync::atomic::AtomicU32;
use std::collections::HashMap;
// note, this implementation is only here to serve HDR's needs and is in no way a definitive file addition solution
// but hell fucking yeah

static mut LOADED_TABLES_OFFSET: usize = 0x0;

static LOADED_TABLES_ADRP_SEARCH_CODE: &[u8] = &[
    0xf3, 0x03, 0x00, 0xaa, 0x1f, 0x01, 0x09, 0x6b, 0xe0, 0x04, 0x00, 0x54,
];

#[repr(C)]
#[repr(packed)]
pub struct Table1Entry {
    pub table2_index: u32,
    pub in_table_2: u32
}

#[repr(C)]
pub struct Table2Entry {
    pub data: *const u8,
    pub ref_count: AtomicU32,
    pub is_used: bool,
    pub state: u8,
    pub file_flags2: bool,
    pub flags: u8,
    pub version: u32,
    pub unk: u8
}

#[repr(C)]
pub struct LoadedData {
    pub arc: &'static mut smash_arc::LoadedArc,
    pub search: &'static mut smash_arc::LoadedSearchSection
}

#[repr(C)]
pub struct LoadedTables {
    pub mutex: *mut skyline::nn::os::MutexType,
    pub table1: *mut Table1Entry,
    pub table2: *mut Table2Entry,
    pub table1_len: u32,
    pub table2_len: u32,
    pub table1_count: u32,
    pub table2_count: u32,
    pub table1_list: [u64; 3], // cppvector
    pub loaded_directory_table: *const u8,
    pub loaded_directory_table_size: u32,
    pub unk2: u32,
    pub unk3: [u64; 3], // cppvector
    pub unk4: u8,
    pub unk5: [u8; 7],
    pub addr: *const (),
    pub loaded_data: &'static mut LoadedData,
    pub version: u32
}

impl LoadedTables {
    pub fn get_instance() -> &'static mut Self {
        unsafe {
            let instance_ptr: *mut &'static mut Self = std::mem::transmute(crate::utils::offset_to_addr(LOADED_TABLES_OFFSET));
            *instance_ptr
        }
    }

    pub fn get_table1() -> &'static [Table1Entry] {
        unsafe {
            let instance = Self::get_instance();
            std::slice::from_raw_parts(instance.table1, instance.table1_len as usize)
        }
    }

    pub fn get_table1_mut() -> &'static mut [Table1Entry] {
        unsafe {
            let instance = Self::get_instance();
            std::slice::from_raw_parts_mut(instance.table1, instance.table1_len as usize)
        }
    }

    pub fn get_table2() -> &'static [Table2Entry] {
        unsafe {
            let instance = Self::get_instance();
            std::slice::from_raw_parts(instance.table2, instance.table2_len as usize)
        }
    }

    pub fn get_table2_mut() -> &'static mut [Table2Entry] {
        unsafe {
            let instance = Self::get_instance();
            std::slice::from_raw_parts_mut(instance.table2, instance.table2_len as usize)
        }
    }

    unsafe fn recreate_array<T: Sized>(start: *const T, length: usize, new_entries: usize) -> *mut T {
        let arr_layout = std::alloc::Layout::from_size_align((length + new_entries) * std::mem::size_of::<T>(), 0x10).unwrap();
        let new_ptr = std::alloc::alloc(arr_layout) as *mut T;
        std::ptr::copy_nonoverlapping(start, new_ptr, length);
        new_ptr
    }

    // Takes a vec of filepaths to add ot the arc's filesystem
    // Returns a hashamp of the filepaths and their new entries
    pub unsafe fn add_files(paths: &Vec<String>) -> HashMap<u64, (u32, u64)> {
        let instance = Self::get_instance();
        skyline::nn::os::LockMutex(instance.mutex);
        let new_entries_num = paths.len();
        let arc: &'static mut smash_arc::LoadedArc = instance.loaded_data.arc;
        let fs_header: &mut smash_arc::FileSystemHeader = std::mem::transmute(arc.fs_header);
        let file_info_path_count = fs_header.file_info_path_count;
        let file_info_index_count = fs_header.file_info_index_count;
        let file_info_indices = std::slice::from_raw_parts(arc.file_info_indices, file_info_index_count as usize);
        let mut last_file = 0;
        for index in file_info_indices.iter() {
            if last_file < index.file_info_index.0 { last_file = index.file_info_index.0; }
        }
        last_file += 1;
        let file_infos = std::slice::from_raw_parts(arc.file_infos, last_file as usize);
        let mut last_info_to_data = 0;
        for info in file_infos.iter() {
            if last_info_to_data < info.info_to_data_index.0 { last_info_to_data = info.info_to_data_index.0; }
        }
        last_info_to_data += 1;
        let info_to_datas = std::slice::from_raw_parts(arc.file_info_to_datas, last_info_to_data as usize);
        let mut last_data = 0;
        for info in info_to_datas.iter() {
            if last_data < info.file_data_index.0 { last_data = info.file_data_index.0; }
        }
        last_data += 1;
        arc.file_info_indices = Self::recreate_array(arc.file_info_indices, file_info_index_count as usize, new_entries_num);
        arc.file_paths = Self::recreate_array(arc.file_paths, file_info_path_count as usize, new_entries_num);
        arc.file_infos = Self::recreate_array(arc.file_infos, last_file as usize, new_entries_num);
        arc.file_info_to_datas = Self::recreate_array(arc.file_info_to_datas, last_info_to_data as usize, new_entries_num);
        arc.file_datas = Self::recreate_array(arc.file_datas, last_data as usize, new_entries_num);
        instance.table1 = Self::recreate_array(instance.table1, instance.table1_len as usize, new_entries_num);
        instance.table2 = Self::recreate_array(instance.table2, instance.table2_len as usize, new_entries_num);
        instance.table1_len += new_entries_num as u32;
        instance.table2_len += new_entries_num as u32;
        fs_header.file_info_path_count += new_entries_num as u32;
        fs_header.file_info_index_count += new_entries_num as u32;
        let file_info_path_count = (*arc.fs_header).file_info_path_count;
        let file_info_index_count = (*arc.fs_header).file_info_index_count;
        let file_paths = std::slice::from_raw_parts_mut(arc.file_paths as *mut smash_arc::FilePath, file_info_path_count as usize);
        let file_info_indices = std::slice::from_raw_parts_mut(arc.file_info_indices as *mut smash_arc::FileInfoIndex, file_info_index_count as usize);
        let file_infos = std::slice::from_raw_parts_mut(arc.file_infos, (last_file + 1) as usize);
        let file_info_to_datas = std::slice::from_raw_parts_mut(arc.file_info_to_datas, (last_info_to_data + 1) as usize);
        let file_datas = std::slice::from_raw_parts_mut(arc.file_datas, (last_data + 1) as usize);
        let table1 = Self::get_table1_mut();
        let table2 = Self::get_table2_mut();
        let mut index_from_back = 0usize;
        let t1_back = (instance.table1_len - 1) as usize;
        let t2_back = (instance.table2_len - 1) as usize;
        let fp_back = file_paths.len() - 1;
        let fii_back = file_info_indices.len() - 1;
        let fi_back = file_infos.len() - 1;
        let fitd_back = file_info_to_datas.len() - 1;
        let fd_back = file_datas.len() - 1;
        let mut ret = HashMap::new();
        for file_path in paths.iter() {
            let file = std::fs::File::open(file_path).expect(&format!("Unable to open new file {} for file addition.", file_path));
            let file_size = file.metadata().unwrap().len();
            file_datas[fd_back - index_from_back].decomp_size = file_size as u32;
            file_info_to_datas[fitd_back - index_from_back].file_data_index = smash_arc::FileDataIdx((fd_back - index_from_back) as u32);
            file_infos[fi_back - index_from_back].info_to_data_index =        smash_arc::InfoToDataIdx((fitd_back - index_from_back) as u32);
            file_info_indices[fii_back - index_from_back].file_info_index =   smash_arc::FileInfoIdx((fi_back - index_from_back) as u32);
            file_paths[fp_back - index_from_back].path.set_index((fii_back - index_from_back) as u32);
            table1[t1_back - index_from_back].in_table_2 = 0xFFFFFFFF;
            table1[t1_back - index_from_back].table2_index = (t2_back - index_from_back) as u32;
            ret.insert(smash::phx::Hash40::new(file_path).hash, ((t1_back - index_from_back) as u32, file_size));
            index_from_back += 1;
        }
        skyline::nn::os::UnlockMutex(instance.mutex);
        ret
    }
}

static INIT: std::sync::Once = std::sync::Once::new();

pub fn init() {
    INIT.call_once(|| {
        unsafe {
            let text_ptr = skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as *const u8;
            let text_size = (skyline::hooks::getRegionAddress(skyline::hooks::Region::Rodata) as usize) - (text_ptr as usize);
            let text = std::slice::from_raw_parts(text_ptr, text_size);
            if let Some(offset) = crate::utils::find_subsequence(text, LOADED_TABLES_ADRP_SEARCH_CODE) {
                let adrp_offset = offset + 12;
                let _adrp_offset = crate::utils::offset_from_adrp(adrp_offset);
                let ldr_offset = crate::utils::offset_from_ldr(adrp_offset + 4);
                LOADED_TABLES_OFFSET = _adrp_offset + ldr_offset;
            } else {
                panic!("Failed to find LoadedTables offset.");
            }
        }
    });
}
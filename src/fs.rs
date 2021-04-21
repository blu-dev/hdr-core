// HDR's filesystem module
use skyline::{libc, nro::NroInfo};
use std::collections::{VecDeque, HashMap};
use std::thread;
use parking_lot::Mutex;
use std::sync::{Arc, mpsc};
use std::io::{BufReader, Read};
use std::sync::{Once, atomic::*};
use lazy_static::lazy_static;
use super::{c_str, debugln};

pub type LoadCallback = fn(String, Vec<u8>);

#[derive(Clone)]
pub struct LoadRequest {
    pub path: String,
    pub callback: LoadCallback
}

impl PartialEq for LoadRequest {
    fn eq(&self, other: &LoadRequest) -> bool {
        self.path == other.path
    }
}

unsafe impl Send for LoadRequest {}
unsafe impl Sync for LoadRequest {}

impl LoadRequest {
    pub fn new<S: Into<String>>(path: S, callback: LoadCallback) -> Self {
        LoadRequest {
            path: path.into(),
            callback: callback
        }
    }
}

struct FileManager {
    master_thread: Option<thread::JoinHandle<()>>,
    terminator: Option<mpsc::Sender<bool>>,
    queue: Arc<Mutex<VecDeque<LoadRequest>>>
}

unsafe impl Send for FileManager {}
unsafe impl Sync for FileManager {}

impl FileManager {
    pub fn new() -> Self {
        let mut ret = FileManager {
            master_thread: None,
            terminator: None,
            queue: Arc::new(Mutex::new(VecDeque::new()))
        };

        let mut queue = ret.queue.clone();
        let (tx, rx) = mpsc::channel::<bool>();
        ret.terminator = Some(tx);
        ret.master_thread = Some(thread::spawn(move || {
            loop {
                // Break if we get the signal
                if rx.try_recv().is_ok() {
                    break;
                }
                let mut locked = queue.lock(); // Acquire the queue
                let front = locked.pop_front(); // Get first in the queue
                drop(locked); // free up mutex
                if front.is_some() {
                    let req = front.unwrap();
                    if !req.path.starts_with("rom:/") && !req.path.starts_with("sd:/") {
                        panic!("Mount name is invalid in path \"{}\"", req.path);
                    }
                    let path = std::path::Path::new(&req.path); // get system path so we can load it
                    if path.is_file() {

                        let data = std::fs::read(path).expect(&format!("Failed to load file \"{}\".", req.path));
                        debugln!("[HDR::FileManager] Loaded file \"{}\"", req.path);
                        (req.callback)(req.path, data);
                    }
                    else {
                        panic!("Unable to find file \"{}\".", req.path);
                    }
                }
                thread::sleep(std::time::Duration::from_millis(50)); // wait 50ms between loops as to not be overkill, can probably increase this even
            }
        }));
        ret
    }

    pub fn queue(&self, requests: &[LoadRequest]) {
        let mut queue = self.queue.lock();
        for req in requests.iter() {
            if !queue.contains(&req) {
                queue.push_back(req.clone());
            }
        }
    }

    pub fn remove(&self, requests: &[LoadRequest]) {
        let mut queue = self.queue.lock();
        for x in 0..requests.len() {
            if queue.contains(unsafe { requests.get_unchecked(x) } ) {
                queue.remove(x);
            }
        }
    }
}

lazy_static! {
    static ref FILE_MANAGER: FileManager = FileManager::new();
    static ref FILE_MAP: Mutex<HashMap<String, Vec<String>>> = Mutex::new(HashMap::new());
    static ref FILES_TO_ADD: Mutex<Vec<String>> = Mutex::new(Vec::new());
    pub static ref ADDED_FILES: Mutex<HashMap<u64, (u32, u64)>> = Mutex::new(HashMap::new());
}

const FILE_MAP_PATH: &'static str = "rom:/hdr/file_map.json";
static INIT: Once = Once::new();

pub fn init() {
    INIT.call_once(|| {
        FILE_MANAGER.queue(&[LoadRequest::new(FILE_MAP_PATH, handle_load_file_map)]);
    });
}

pub fn load_associated_files(info: &NroInfo) {
    let file_map = FILE_MAP.lock();
    let name = String::from(info.name);
    if name == "common" {
        unsafe {
            let mut to_add = FILES_TO_ADD.lock();
            let mut added_files = ADDED_FILES.lock();
            *added_files = crate::arc_runtime::LoadedTables::add_files(&to_add);
            to_add.clear();
        }
    }
    if file_map.contains_key(&name) {
        let files = file_map.get(&name).expect("File map does not contain module entry.");
        FILE_MANAGER.queue(files.iter().map(|x| LoadRequest::new(x, handle_load_file)).collect::<Vec<LoadRequest>>().as_slice());
    }
}

fn handle_load_file_map(path: String, data: Vec<u8>) {
    use serde_json::{self, *};
    assert!(path == FILE_MAP_PATH);
    let json = std::str::from_utf8(data.as_slice()).expect("The loaded file map is invalid UTF-8 data.");
    let json: Value = serde_json::from_str(json).expect("Unable to parse file map!");
    let mut file_map = FILE_MAP.lock();
    if let Value::Object(module_map) = json {
        for (module, files) in module_map.iter() {
            // Check if the file map contains, if not make a new one that we can add to
            if !file_map.contains_key(module) { file_map.insert(module.clone(), Vec::new()); }
            if let Value::Array(paths) = files {
                let file_paths = file_map.get_mut(module).expect("File map does not contain module entry.");
                let new_file_paths = paths.iter().map(|x| String::from(x.as_str().expect("File map does not contain valid filepath."))).collect::<Vec<String>>();
                for path in new_file_paths.iter() {
                    if path.ends_with(".nuanmb") {
                        let mut to_add = FILES_TO_ADD.lock();
                        to_add.push(path.clone());
                    }
                }
                file_paths.extend(new_file_paths.into_iter());
            } else {
                panic!("File map is invalid JSON for HDR -- Error in module {}.", module);
            }
        }
    } else {
        panic!("File map is invalid JSON for HDR -- File map is not an object.");
    }
}

fn handle_load_file(path: String, data: Vec<u8>) {
    if path.ends_with(".prc") {
        super::modules::param::ParamModule::handle_param_load(path, data);
    } else if path.ends_with(".nuanmb") {
        super::modules::anim::handle_nuanmb_load(path, data);
    }
}
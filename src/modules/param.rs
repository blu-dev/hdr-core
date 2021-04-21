use smash::app::BattleObjectModuleAccessor;
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::io::Cursor;
use lazy_static::lazy_static;
use super::PARAM_MODULE_OFFSET;
use crate::debugln;

macro_rules! get_param_module {
    ($boma:ident) => {{
        let vtable = *($boma as *const *const u64);
        &mut *(*vtable.offset((*vtable.offset(-1) as isize) + PARAM_MODULE_OFFSET) as *mut ParamModule)
    }}
}

// Basic principle
// 1. When the game starts, load our common params into memory. Store them, allowing each new 
//    ParamModule to take an Arc of the common params. This allows us to maintain a single instance
//    for the lifetime of the program that every fighter can reference
// 2. When a fighter NRO is loaded, we queue the files over the in the `fs` module. We read the
//    file extension and if it's a `.prc` file we send it to the ParamModule
// 3. When the ParamModule gets a fighter PRC, we use prc-rs to parse it. For our use case, since we
//    are just beginning to use custom param files, I (blujay) think it is acceptable to limit
//    each fighter's params to the highest level (no nested structs, no lists except for the shared fighter params)
// 4. When the fighter NRO is unloaded, we first remove any potential loads from the queue, since that will block
//    the calling thread until we know for sure that the files have either been loaded or prevented from being loaded,
//    and then we signal to ParamModule that it can release the static references to our parsed param data.
// Note:
//    Since I have not yet tested this, I am unsure of how much delay it will cause in-game when loading a match, but it shouldn't
//    stall the UI thread since the NRO loading thread is separate from that. Ideally we don't have to implement a separate parsing
//    thread for the parameter data, but if we need to we can.

lazy_static! {
    static ref COMMON_INT:   RwLock<Option<Arc<HashMap<u64, i32>>>>  = RwLock::new(None);
    static ref COMMON_INT64: RwLock<Option<Arc<HashMap<u64, u64>>>>  = RwLock::new(None);
    static ref COMMON_FLOAT: RwLock<Option<Arc<HashMap<u64, f32>>>>  = RwLock::new(None);
    static ref COMMON_FLAG:  RwLock<Option<Arc<HashMap<u64, bool>>>> = RwLock::new(None);

    static ref SHARED_FIGHTER_INT:   RwLock<Option<Vec<Arc<HashMap<u64, i32>>>>>  = RwLock::new(None);
    static ref SHARED_FIGHTER_INT64: RwLock<Option<Vec<Arc<HashMap<u64, u64>>>>>  = RwLock::new(None);
    static ref SHARED_FIGHTER_FLOAT: RwLock<Option<Vec<Arc<HashMap<u64, f32>>>>>  = RwLock::new(None);
    static ref SHARED_FIGHTER_FLAG:  RwLock<Option<Vec<Arc<HashMap<u64, bool>>>>> = RwLock::new(None);

    static ref AGENT_INT:   RwLock<HashMap<String, Arc<HashMap<u64, i32>>>>  = RwLock::new(HashMap::new());
    static ref AGENT_INT64: RwLock<HashMap<String, Arc<HashMap<u64, u64>>>>  = RwLock::new(HashMap::new());
    static ref AGENT_FLOAT: RwLock<HashMap<String, Arc<HashMap<u64, f32>>>>  = RwLock::new(HashMap::new());
    static ref AGENT_FLAG:  RwLock<HashMap<String, Arc<HashMap<u64, bool>>>> = RwLock::new(HashMap::new());
}

pub enum ParamType {
    Common,
    Shared,
    Agent
}

pub struct ParamModule {
    common_int: Option<Arc<HashMap<u64, i32>>>,
    common_int64: Option<Arc<HashMap<u64, u64>>>,
    common_float: Option<Arc<HashMap<u64, f32>>>,
    common_flag: Option<Arc<HashMap<u64, bool>>>,

    shared_int: Option<Arc<HashMap<u64, i32>>>,
    shared_int64: Option<Arc<HashMap<u64, u64>>>,
    shared_float: Option<Arc<HashMap<u64, f32>>>,
    shared_flag: Option<Arc<HashMap<u64, bool>>>,

    agent_int: Option<Arc<HashMap<u64, i32>>>,
    agent_int64: Option<Arc<HashMap<u64, u64>>>,
    agent_float: Option<Arc<HashMap<u64, f32>>>,
    agent_flag: Option<Arc<HashMap<u64, bool>>>,
}

impl ParamModule {
    fn handle_common_prc(obj: &prc::ParamStruct) {
        let mut int = COMMON_INT.write();
        let mut int64 = COMMON_INT64.write();
        let mut float = COMMON_FLOAT.write();
        let mut flag = COMMON_FLAG.write();
        assert!(int.is_none() && int64.is_none() && float.is_none() && flag.is_none(), "Error: Common PRC Reloaded");
        let mut int_map = HashMap::<u64, i32>::new();
        let mut int64_map = HashMap::<u64, u64>::new();
        let mut float_map = HashMap::<u64, f32>::new();
        let mut flag_map = HashMap::<u64, bool>::new();
        if let prc::ParamStruct(params) = obj {
            for (hash, value) in params.iter() {
                use prc::ParamKind::*;
                let hash = if let prc::hash40::Hash40(val) = hash { val } else { unreachable!() };
                match value {
                    Bool(val) => {
                        flag_map.insert(*hash, *val);
                    },
                    I8(val) => {
                        int_map.insert(*hash,*val as i32);
                    },
                    U8(val) => {
                        int_map.insert(*hash,*val as i32);
                    },
                    I16(val) => {
                        int_map.insert(*hash,*val as i32);
                    },
                    U16(val) => {
                        int_map.insert(*hash,*val as i32);
                    },
                    I32(val) => {
                        int_map.insert(*hash,*val);
                    },
                    U32(val) => {
                        int_map.insert(*hash,*val as i32);
                    },
                    Float(val) => {
                        float_map.insert(*hash, *val);
                    },
                    Hash(val) => {
                        let val = if let prc::hash40::Hash40(v) = val { v } else { unreachable!() };
                        int64_map.insert(*hash, *val);
                    },
                    _ => {
                        panic!("Invalid param kind: must be bool, int, int64, or float.");
                    }
                }
            }
        } else { unreachable!() }
        *int = Some(Arc::new(int_map));
        *int64 = Some(Arc::new(int64_map));
        *float = Some(Arc::new(float_map));
        *flag = Some(Arc::new(flag_map));
    }

    fn handle_shared_prc(obj: &prc::ParamStruct) {
        let mut int = SHARED_FIGHTER_INT.write();
        let mut int64 = SHARED_FIGHTER_INT64.write();
        let mut float = SHARED_FIGHTER_FLOAT.write();
        let mut flag = SHARED_FIGHTER_FLAG.write();
        assert!(int.is_none() && int64.is_none() && float.is_none() && flag.is_none(), "Error: Shared fighter PRC reloaded.");
        if let prc::ParamStruct(params) = obj {
            use prc::ParamKind::*;
            assert!(params.len() == 1, "Error: Shared fighter PRC has the wrong amount of elements.");
            if let (_, List(list)) = params.get(0).expect("Error: Failed to read parsed PRC data.") {
            if let prc::ParamList(list) = list {
                let sz = list.len();
                let mut int_vec = Vec::with_capacity(sz);
                let mut int64_vec = Vec::with_capacity(sz);
                let mut float_vec = Vec::with_capacity(sz);
                let mut flag_vec = Vec::with_capacity(sz);
                for param in list.iter() {
                if let Struct(param) = param {
                if let prc::ParamStruct(fighter_params) = param {
                    let mut int_map = HashMap::<u64, i32>::new();
                    let mut int64_map = HashMap::<u64, u64>::new();
                    let mut float_map = HashMap::<u64, f32>::new();
                    let mut flag_map = HashMap::<u64, bool>::new();
                    for (hash, value) in fighter_params.iter() {
                        let hash = if let prc::hash40::Hash40(val) = hash { val } else { unreachable!() };
                        match value {
                            Bool(val) => {
                                flag_map.insert(*hash, *val);
                            },
                            I8(val) => {
                                int_map.insert(*hash,*val as i32);
                            },
                            U8(val) => {
                                int_map.insert(*hash,*val as i32);
                            },
                            I16(val) => {
                                int_map.insert(*hash,*val as i32);
                            },
                            U16(val) => {
                                int_map.insert(*hash,*val as i32);
                            },
                            I32(val) => {
                                int_map.insert(*hash,*val);
                            },
                            U32(val) => {
                                int_map.insert(*hash,*val as i32);
                            },
                            Float(val) => {
                                float_map.insert(*hash, *val);
                            },
                            Hash(val) => {
                                let val = if let prc::hash40::Hash40(v) = val { v } else { unreachable!() };
                                int64_map.insert(*hash, *val);
                            },
                            _ => {
                                panic!("Invalid param kind: must be bool, int, int64, or float.");
                            }
                        }
                    }
                    int_vec.push(Arc::new(int_map));
                    int64_vec.push(Arc::new(int64_map));
                    float_vec.push(Arc::new(float_map));
                    flag_vec.push(Arc::new(flag_map));
                } else { unreachable!() }
                } else { panic!("Error: Malformed shared fighter PRC."); }
                }
                *int = Some(int_vec);
                *int64 = Some(int64_vec);
                *float = Some(float_vec);
                *flag = Some(flag_vec);
            } else { unreachable!() }
            } else { panic!("Error: Malformed shared fighter PRC."); }
        } else { unreachable!() }
    }

    fn handle_fighter_prc(agent: &String, obj: &prc::ParamStruct) {
        let mut int = AGENT_INT.write();
        let mut int64 = AGENT_INT64.write();
        let mut float = AGENT_FLOAT.write();
        let mut flag = AGENT_FLAG.write();
        assert!(!int.contains_key(agent) && !int64.contains_key(agent) && !float.contains_key(agent) && !flag.contains_key(agent),
            "Error: Unique fighter PRC reloaded while previous is still loaded.");
        let mut int_map = HashMap::<u64, i32>::new();
        let mut int64_map = HashMap::<u64, u64>::new();
        let mut float_map = HashMap::<u64, f32>::new();
        let mut flag_map = HashMap::<u64, bool>::new();
        if let prc::ParamStruct(params) = obj {
            for (hash, value) in params.iter() {
                use prc::ParamKind::*;
                let hash = if let prc::hash40::Hash40(val) = hash { val } else { unreachable!() };
                match value {
                    Bool(val) => {
                        flag_map.insert(*hash, *val);
                    },
                    I8(val) => {
                        int_map.insert(*hash,*val as i32);
                    },
                    U8(val) => {
                        int_map.insert(*hash,*val as i32);
                    },
                    I16(val) => {
                        int_map.insert(*hash,*val as i32);
                    },
                    U16(val) => {
                        int_map.insert(*hash,*val as i32);
                    },
                    I32(val) => {
                        int_map.insert(*hash,*val);
                    },
                    U32(val) => {
                        int_map.insert(*hash,*val as i32);
                    },
                    Float(val) => {
                        float_map.insert(*hash, *val);
                    },
                    Hash(val) => {
                        let val = if let prc::hash40::Hash40(v) = val { v } else { unreachable!() };
                        int64_map.insert(*hash, *val);
                    },
                    _ => {
                        panic!("Invalid param kind: must be bool, int, int64, or float.");
                    }
                }
            }
        } else { unreachable!() }
        int.insert(agent.clone(), Arc::new(int_map));
        int64.insert(agent.clone(), Arc::new(int64_map));
        float.insert(agent.clone(), Arc::new(float_map));
        flag.insert(agent.clone(), Arc::new(flag_map));
    }

    fn _get_int(&self, ty: ParamType, hash: u64) -> i32 {
        match ty {
            ParamType::Common => {
                if let Some(map) = self.common_int.as_ref() {
                    *map.get(&hash).unwrap_or(&0)
                } else {
                    i32::default()
                }
            },
            ParamType::Shared => {
                if let Some(map) = self.shared_int.as_ref() {
                    *map.get(&hash).unwrap_or(&0)
                } else {
                    i32::default()
                }
            },
            ParamType::Agent => {
                if let Some(map) = self.agent_int.as_ref() {
                    *map.get(&hash).unwrap_or(&0)
                } else {
                    i32::default()
                }
            },
            _ => { unreachable!() }
        }
    }

    fn _get_int64(&self, ty: ParamType, hash: u64) -> u64 {
        match ty {
            ParamType::Common => {
                if let Some(map) = self.common_int64.as_ref() {
                    *map.get(&hash).unwrap_or(&0)
                } else {
                    u64::default()
                }
            },
            ParamType::Shared => {
                if let Some(map) = self.shared_int64.as_ref() {
                    *map.get(&hash).unwrap_or(&0)
                } else {
                    u64::default()
                }
            },
            ParamType::Agent => {
                if let Some(map) = self.agent_int64.as_ref() {
                    *map.get(&hash).unwrap_or(&0)
                } else {
                    u64::default()
                }
            },
            _ => { unreachable!() }
        }
    }

    fn _get_float(&self, ty: ParamType, hash: u64) -> f32 {
        match ty {
            ParamType::Common => {
                if let Some(map) = self.common_float.as_ref() {
                    *map.get(&hash).unwrap_or(&0.0)
                } else {
                    f32::default()
                }
            },
            ParamType::Shared => {
                if let Some(map) = self.shared_float.as_ref() {
                    *map.get(&hash).unwrap_or(&0.0)
                } else {
                    f32::default()
                }
            },
            ParamType::Agent => {
                if let Some(map) = self.agent_float.as_ref() {
                    *map.get(&hash).unwrap_or(&0.0)
                } else {
                    f32::default()
                }
            },
            _ => { unreachable!() }
        }
    }

    fn _get_flag(&self, ty: ParamType, hash: u64) -> bool {
        match ty {
            ParamType::Common => {
                if let Some(map) = self.common_flag.as_ref() {
                    *map.get(&hash).unwrap_or(&false)
                } else {
                    false
                }
            },
            ParamType::Shared => {
                if let Some(map) = self.shared_flag.as_ref() {
                    *map.get(&hash).unwrap_or(&false)
                } else {
                    false
                }
            },
            ParamType::Agent => {
                if let Some(map) = self.agent_flag.as_ref() {
                    *map.get(&hash).unwrap_or(&false)
                } else {
                    false
                }
            },
            _ => { unreachable!() }
        }
    }

    pub(crate) fn handle_param_load(path: String, data: Vec<u8>) {
        assert!(path.ends_with(".prc"), "ParamModule cannot handle non-param data types.");
        // probably a better way to handle this but I'm not interested at the moment
        if path.starts_with("rom:/hdr/common/") {
            if path.ends_with("common.prc") {
                let mut buf = Cursor::new(data);
                let parsed = prc::read_stream(&mut buf).expect("Could not parse HDR's common.prc");
                Self::handle_common_prc(&parsed);
            } else if path.ends_with("fighter_param.prc") {
                let mut buf = Cursor::new(data);
                let parsed = prc::read_stream(&mut buf).expect("Could not parse HDR's fighter_param.prc");
                Self::handle_shared_prc(&parsed);
            } else {
                panic!("Common param file loaded that is not handled.");
            }
        } else {
            let tokens: Vec<String> = path.split('/').map(|x| String::from(x)).collect();
            let agent = tokens.get(2).expect("Invalid unique fighter param path.");
            let mut buf = Cursor::new(data);
            let parsed = prc::read_stream(&mut buf).expect("Could not parse fighter's param file.");
            Self::handle_fighter_prc(agent, &parsed);
        }
        debugln!("loaded {}", path);
    }

    pub(crate) fn handle_param_unload(info: &skyline::nro::NroInfo) {
        let module = &String::from(info.name);
        let mut int = AGENT_INT.write();
        let mut int64 = AGENT_INT64.write();
        let mut float = AGENT_FLOAT.write();
        let mut flag = AGENT_FLAG.write();
        int.remove(module);
        int64.remove(module);
        float.remove(module);
        flag.remove(module);
    }

    pub fn new(category: i32, agent_kind: i32) -> Self {
        unsafe {
            let mut ret = Self {
                common_int: None,
                common_int64: None,
                common_float: None,
                common_flag: None,
                shared_int: None,
                shared_int64: None,
                shared_float: None,
                shared_flag: None,
                agent_int: None,
                agent_int64: None,
                agent_float: None,
                agent_flag: None
            };
            if category == *smash::lib::lua_const::BATTLE_OBJECT_CATEGORY_FIGHTER && !cfg!(feature = "no_common_params") {
                let int = COMMON_INT.read();
                let int64 = COMMON_INT64.read();
                let float = COMMON_FLOAT.read();
                let flag = COMMON_FLAG.read();
                ret.common_int = Some(int.clone().expect("Common prc not loaded.").clone());
                ret.common_int64 = Some(int64.clone().expect("Common prc not loaded.").clone());
                ret.common_float = Some(float.clone().expect("Common prc not loaded.").clone());
                ret.common_flag = Some(flag.clone().expect("Common prc not loaded.").clone());
    
                let int = SHARED_FIGHTER_INT.read();
                let int64 = SHARED_FIGHTER_INT64.read();
                let float = SHARED_FIGHTER_FLOAT.read();
                let flag = SHARED_FIGHTER_FLAG.read();
                ret.shared_int = Some(int.clone().expect("Common prc not loaded.").get(agent_kind as usize).expect("Fighter missing from fighter params").clone());
                ret.shared_int64 = Some(int64.clone().expect("Common prc not loaded.").get(agent_kind as usize).expect("Fighter missing from fighter params").clone());
                ret.shared_float = Some(float.clone().expect("Common prc not loaded.").get(agent_kind as usize).expect("Fighter missing from fighter params").clone());
                ret.shared_flag = Some(flag.clone().expect("Common prc not loaded.").get(agent_kind as usize).expect("Fighter missing from fighter params").clone());
            }

            let int = AGENT_INT.read();
            let int64 = AGENT_INT64.read();
            let float = AGENT_FLOAT.read();
            let flag = AGENT_FLAG.read();
            let mut _agent = None;
            for (agent, params) in int.iter() {
                if crate::utils::agent_to_agent_kind(agent) == agent_kind {
                    _agent = Some(agent.clone());
                    break;
                }
            }
            if _agent.is_some() {
                let agent = _agent.unwrap();
                ret.agent_int = Some(int.get(&agent).expect("Invalid loaded PRC state.").clone());
                ret.agent_int64 = Some(int64.get(&agent).expect("Invalid loaded PRC state.").clone());
                ret.agent_float = Some(float.get(&agent).expect("Invalid loaded PRC state.").clone());
                ret.agent_flag = Some(flag.get(&agent).expect("Invalid loaded PRC state.").clone());
            }

            ret
        }
    }

    #[cfg_attr(feature = "debug", export_name = "ParamModule__get_int")]
    pub fn get_int(boma: *mut smash::app::BattleObjectModuleAccessor, ty: ParamType, string: &str) -> i32 {
        unsafe {
            get_param_module!(boma)._get_int(ty, smash::phx::Hash40::new(string).hash)
        }
    }

    #[cfg_attr(feature = "debug", export_name = "ParamModule__get_int64")]
    pub fn get_int64(boma: *mut smash::app::BattleObjectModuleAccessor, ty: ParamType, string: &str) -> u64 {
        unsafe {
            get_param_module!(boma)._get_int64(ty, smash::phx::Hash40::new(string).hash)
        }
    }

    #[cfg_attr(feature = "debug", export_name = "ParamModule__get_float")]
    pub fn get_float(boma: *mut smash::app::BattleObjectModuleAccessor, ty: ParamType, string: &str) -> f32 {
        unsafe {
            get_param_module!(boma)._get_float(ty, smash::phx::Hash40::new(string).hash)
        }
    }

    #[cfg_attr(feature = "debug", export_name = "ParamModule__get_flag")]
    pub fn get_flag(boma: *mut smash::app::BattleObjectModuleAccessor, ty: ParamType, string: &str) -> bool {
        unsafe {
            get_param_module!(boma)._get_flag(ty, smash::phx::Hash40::new(string).hash)
        }
    }
}
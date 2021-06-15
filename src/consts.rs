use std::cell::UnsafeCell;
use std::ops::Deref;
use std::cmp::{PartialEq, PartialOrd, Ordering as CmpOrdering};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, Ordering};
use parking_lot::Mutex;

pub type GeneratorFn = fn(u32) -> i32;

lazy_static! {
    static ref RUNTIME_CONSTS: Mutex<HashMap<u32, (HashMap<u32, i32>, Option<GeneratorFn>)>> = Mutex::new(HashMap::new());
}

pub struct RuntimeConstant {
    pub value: UnsafeCell<Option<i32>>,
    pub category: u32,
    pub name: u32
}

unsafe impl Send for RuntimeConstant {}
unsafe impl Sync for RuntimeConstant {}

impl Deref for RuntimeConstant {
    type Target = i32;

    fn deref(&self) -> &Self::Target {
        unsafe {
            let val = self.value.get();
            if let Some(ref val) = *val {
                // this is gross but ok jam
                std::mem::transmute(val)
            } else {
                *val = Some(RuntimeConstant::resolver(self));
                (*val).as_ref().unwrap()
            }
        }
    }
}

macro_rules! equality_impls {
    ($($ty:ty)*) => {
        $(
            impl PartialEq<$ty> for RuntimeConstant {
                fn eq(&self, other: &$ty) -> bool {
                    return **self == other.clone() as i32;
                }
            }

            impl PartialOrd<$ty> for RuntimeConstant {
                fn partial_cmp(&self, other: &$ty) -> Option<CmpOrdering> {
                    Some((**self).cmp(&(other.clone() as i32)))
                }
            }

            impl PartialEq<RuntimeConstant> for $ty {
                fn eq(&self, other: &RuntimeConstant) -> bool {
                    return **other == *self as i32;
                }
            }

            impl PartialOrd<RuntimeConstant> for $ty {
                fn partial_cmp(&self, other: &RuntimeConstant) -> Option<CmpOrdering> {
                    Some((*self as i32).cmp(&**other))
                }
            }
        )*
    }
}

impl PartialEq for RuntimeConstant {
    fn eq(&self, other: &RuntimeConstant) -> bool {
        *self == **other
    }
}

equality_impls!(i32 u32 u64);

fn default_generator(category: u32) -> i32 {
    lazy_static! {
        pub static ref DEFAULT_MAP: Mutex<HashMap<u32, AtomicI32>> = Mutex::new(HashMap::new());
    }

    let mut map = DEFAULT_MAP.lock();
    if let Some(val) = map.get_mut(&category) {
        val.fetch_add(1, Ordering::SeqCst)
    } else {
        let _ = map.try_insert(category, AtomicI32::new(0)); // ignore result here
        if let Some(val) = map.get_mut(&category) {
            val.fetch_add(1, Ordering::SeqCst)
        } else {
            unreachable!()
        }
    }
}

impl RuntimeConstant {
    #[export_name = "hdr_constant_resolver"]
    pub fn resolver(c: &RuntimeConstant) -> i32 {
        let mut runtime_consts = RUNTIME_CONSTS.lock();
        if let Some((set, generator)) = runtime_consts.get_mut(&c.category) {
            if let Some(val) = set.get(&c.name) {
                *val
            } else {
                let f = generator.clone().unwrap();
                let val = f(c.category);
                set.insert(c.name, val);
                val
            }
        } else {
            println!("[hdr-core] Resolver called on constant with missing category, defaulting to -1.");
            -1
        }
    }
}

#[export_name = "hdr_constant_add_category"]
pub fn add_category(category: u32, mut generator: Option<GeneratorFn>) {
    let mut runtime_consts = RUNTIME_CONSTS.lock();
    if runtime_consts.contains_key(&category) {
        panic!("HDR-Core: Constant category {:#x} already exists.", category);
    }
    if generator.is_none() {
        generator = Some(default_generator);
    }
    runtime_consts.insert(category, (HashMap::new(), generator));
}
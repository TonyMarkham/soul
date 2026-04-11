use abi_stable::{
    StableAbi,
    std_types::{RHashMap, RString},
};

#[repr(C)]
#[derive(StableAbi, Debug, Clone)]
pub struct NormalizedAnnotation {
    pub id: RString,
    pub metadata: RHashMap<RString, RString>,
    pub raw: RString,
}

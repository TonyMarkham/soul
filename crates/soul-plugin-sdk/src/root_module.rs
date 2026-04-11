use crate::parser::AnnotationParser_TO;

use abi_stable::{
    StableAbi, library::RootModule, package_version_strings, sabi_types::VersionStrings,
    std_types::RBox,
};

#[repr(C)]
#[derive(StableAbi)]
#[sabi(kind(Prefix(prefix_ref = SoulPluginRef)))]
#[sabi(missing_field(panic))]
pub struct SoulPlugin {
    #[sabi(last_prefix_field)]
    pub parser: extern "C" fn() -> AnnotationParser_TO<'static, RBox<()>>,
}

impl RootModule for SoulPluginRef {
    abi_stable::declare_root_module_statics! {SoulPluginRef}
    const BASE_NAME: &'static str = "soul_plugin";
    const NAME: &'static str = "soul_plugin";
    const VERSION_STRINGS: VersionStrings = package_version_strings!();
}

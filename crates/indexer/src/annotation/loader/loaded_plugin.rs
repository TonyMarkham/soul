use soul_plugin_sdk::AnnotationParser_TO;

use abi_stable::std_types::RBox;

pub struct LoadedPlugin {
    pub language: String,
    pub parser: AnnotationParser_TO<'static, RBox<()>>,
}

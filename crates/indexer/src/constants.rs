pub const SOUL_DIR: &str = ".soul";
pub const CONFIG_FILE: &str = "soul.toml";
pub const INDEX_FILE: &str = "index.db";
pub const EXAMPLE_CONFIG: &str = include_str!("../../../soul.toml.example");
pub const SCAN_CONFIG_BASE: &str = "\
[scan]
excluded_dirs = [\".git\", \".soul\", \"target\", \".idea\", \".vscode\", \".vs\", \".codex\", \"node_modules\", \"obj\"]
excluded_dir_suffixes = [\"Tests\", \".Tests\", \"tests\", \".tests\"]
excluded_bin_except_under = [\"src\"]
";

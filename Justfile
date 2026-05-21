set shell := ["sh", "-cu"]
set windows-shell := ["pwsh.exe", "-NoLogo", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command"]

dist := "dist"

default:
    just --list

show-platform:
    @just print-platform-{{os()}}-{{arch()}}

print-platform-linux-x86_64:
    @echo "linux-x64"

print-platform-linux-aarch64:
    @echo "linux-arm64"

print-platform-macos-aarch64:
    @echo "macos-arm64"

print-platform-windows-x86_64:
    @echo "windows-x64"

test:
    cargo test --workspace

build:
    cargo build -p indexer -p soul-lsp -p soul-plugin-rust -p soul-plugin-csharp --release

write-soul-config-unix root ext:
    printf '%s\n' '[scan]' 'excluded_dirs = [".git", ".soul", "target", ".idea", ".vscode", ".vs", ".codex", "node_modules", "obj", ".archive", ".claude"]' 'excluded_dir_suffixes = ["Tests", ".Tests", "tests", ".tests"]' 'excluded_bin_except_under = ["src"]' '' '# Plugins declare which file extensions they handle.' '# Paths are platform-specific (.dylib on macOS, .so on Linux, .dll on Windows).' '[[plugins]]' 'language = "rust"' 'path = "./.soul/plugins/rust{{ext}}"' '' '[[plugins]]' 'language = "csharp"' 'path = "./.soul/plugins/csharp{{ext}}"' > "{{root}}/.soul/soul.toml"

write-soul-config-windows root:
    @('[scan]', 'excluded_dirs = [".git", ".soul", "target", ".idea", ".vscode", ".vs", ".codex", "node_modules", "obj", ".archive", ".claude"]', 'excluded_dir_suffixes = ["Tests", ".Tests", "tests", ".tests"]', 'excluded_bin_except_under = ["src"]', '', '# Plugins declare which file extensions they handle.', '# Paths are platform-specific (.dylib on macOS, .so on Linux, .dll on Windows).', '[[plugins]]', 'language = "rust"', 'path = "./.soul/plugins/rust.dll"', '', '[[plugins]]', 'language = "csharp"', 'path = "./.soul/plugins/csharp.dll"') | Set-Content -LiteralPath '{{root}}\.soul\soul.toml' -Encoding UTF8

check-linux-x64:
    test "$(rustc -vV | awk '/host:/ {print $2}')" = "x86_64-unknown-linux-gnu"

check-macos-arm64:
    test "$(rustc -vV | awk '/host:/ {print $2}')" = "aarch64-apple-darwin"

check-linux-arm64:
    test "$(rustc -vV | awk '/host:/ {print $2}')" = "aarch64-unknown-linux-gnu"

check-windows-x64:
    $HostTriple = (rustc -vV | Select-String '^host:' | ForEach-Object { ($_.Line -split '\s+')[1] }); if ($HostTriple -ne 'x86_64-pc-windows-msvc') { throw "Expected x86_64-pc-windows-msvc, got $HostTriple" }

archive-linux-x64: check-linux-x64 build
    rm -rf {{dist}}/soul-linux-x64 {{dist}}/soul-linux-x64.tar.gz
    mkdir -p {{dist}}/soul-linux-x64/.soul/plugins
    cp target/release/indexer {{dist}}/soul-linux-x64/.soul/indexer
    cp target/release/soul-lsp {{dist}}/soul-linux-x64/.soul/soul-lsp
    cp .soul/.gitignore {{dist}}/soul-linux-x64/.soul/.gitignore
    cp -R .soul/templates {{dist}}/soul-linux-x64/.soul/templates
    cp target/release/libsoul_plugin_rust.so {{dist}}/soul-linux-x64/.soul/plugins/rust.so
    cp target/release/libsoul_plugin_csharp.so {{dist}}/soul-linux-x64/.soul/plugins/csharp.so
    just write-soul-config-unix {{dist}}/soul-linux-x64 .so
    tar -C {{dist}}/soul-linux-x64 -czf {{dist}}/soul-linux-x64.tar.gz .soul

archive-macos-arm64: check-macos-arm64 build
    rm -rf {{dist}}/soul-macos-arm64 {{dist}}/soul-macos-arm64.tar.gz
    mkdir -p {{dist}}/soul-macos-arm64/.soul/plugins
    cp target/release/indexer {{dist}}/soul-macos-arm64/.soul/indexer
    cp target/release/soul-lsp {{dist}}/soul-macos-arm64/.soul/soul-lsp
    cp .soul/.gitignore {{dist}}/soul-macos-arm64/.soul/.gitignore
    cp -R .soul/templates {{dist}}/soul-macos-arm64/.soul/templates
    cp target/release/libsoul_plugin_rust.dylib {{dist}}/soul-macos-arm64/.soul/plugins/rust.dylib
    cp target/release/libsoul_plugin_csharp.dylib {{dist}}/soul-macos-arm64/.soul/plugins/csharp.dylib
    just write-soul-config-unix {{dist}}/soul-macos-arm64 .dylib
    tar -C {{dist}}/soul-macos-arm64 -czf {{dist}}/soul-macos-arm64.tar.gz .soul

archive-linux-arm64: check-linux-arm64 build
    rm -rf {{dist}}/soul-linux-arm64 {{dist}}/soul-linux-arm64.tar.gz
    mkdir -p {{dist}}/soul-linux-arm64/.soul/plugins
    cp target/release/indexer {{dist}}/soul-linux-arm64/.soul/indexer
    cp target/release/soul-lsp {{dist}}/soul-linux-arm64/.soul/soul-lsp
    cp .soul/.gitignore {{dist}}/soul-linux-arm64/.soul/.gitignore
    cp -R .soul/templates {{dist}}/soul-linux-arm64/.soul/templates
    cp target/release/libsoul_plugin_rust.so {{dist}}/soul-linux-arm64/.soul/plugins/rust.so
    cp target/release/libsoul_plugin_csharp.so {{dist}}/soul-linux-arm64/.soul/plugins/csharp.so
    just write-soul-config-unix {{dist}}/soul-linux-arm64 .so
    tar -C {{dist}}/soul-linux-arm64 -czf {{dist}}/soul-linux-arm64.tar.gz .soul

archive-windows-x64: check-windows-x64 build
    if (Test-Path -LiteralPath '{{dist}}\soul-windows-x64') { Remove-Item -LiteralPath '{{dist}}\soul-windows-x64' -Recurse -Force }
    if (Test-Path -LiteralPath '{{dist}}\soul-windows-x64.zip') { Remove-Item -LiteralPath '{{dist}}\soul-windows-x64.zip' -Force }
    New-Item -ItemType Directory -Path '{{dist}}\soul-windows-x64\.soul\plugins' -Force | Out-Null
    Copy-Item 'target\release\indexer.exe' '{{dist}}\soul-windows-x64\.soul\indexer.exe'
    Copy-Item 'target\release\soul-lsp.exe' '{{dist}}\soul-windows-x64\.soul\soul-lsp.exe'
    Copy-Item '.soul\.gitignore' '{{dist}}\soul-windows-x64\.soul\.gitignore'
    Copy-Item '.soul\templates' '{{dist}}\soul-windows-x64\.soul\templates' -Recurse
    Copy-Item 'target\release\soul_plugin_rust.dll' '{{dist}}\soul-windows-x64\.soul\plugins\rust.dll'
    Copy-Item 'target\release\soul_plugin_csharp.dll' '{{dist}}\soul-windows-x64\.soul\plugins\csharp.dll'
    just write-soul-config-windows '{{dist}}\soul-windows-x64'
    Compress-Archive -Path '{{dist}}\soul-windows-x64\.soul' -DestinationPath '{{dist}}\soul-windows-x64.zip' -Force

archive:
    just archive-{{os()}}-{{arch()}}

archive-linux-x86_64: archive-linux-x64

archive-linux-aarch64: archive-linux-arm64

archive-macos-aarch64: archive-macos-arm64

archive-windows-x86_64: archive-windows-x64

upload tag:
    just upload-{{os()}}-{{arch()}} {{tag}}

upload-linux-x86_64 tag:
    gh release upload {{tag}} {{dist}}/soul-linux-x64.tar.gz --clobber

upload-linux-aarch64 tag:
    gh release upload {{tag}} {{dist}}/soul-linux-arm64.tar.gz --clobber

upload-macos-aarch64 tag:
    gh release upload {{tag}} {{dist}}/soul-macos-arm64.tar.gz --clobber

upload-windows-x86_64 tag:
    gh release upload {{tag}} {{dist}}/soul-windows-x64.zip --clobber

release tag: archive
    just upload {{tag}}

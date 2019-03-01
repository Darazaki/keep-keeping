# Just

install_dir = "/Applications/Keep Keeping.app/Contents/MacOS/Keep Keeping"

# Build and install (default)
@install:
    echo "\n=== Building keep_keeping in release mode ===\n"
    cargo build --release
    echo "\n=== Installing keep_keeping ===\n"
    cp -f "target/release/keep_keeping" "{{install_dir}}"
    echo "    Finished with exit code 0"
    echo "\n==> Finished, binary installed at '{{install_dir}}'"

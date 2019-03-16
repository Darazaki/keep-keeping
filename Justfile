# Just

install_dir = "/Applications/Keep Keeping.app/Contents/MacOS/Keep Keeping"

# Build CLI and GUI (default)
@build-all +args='':
    just build-cli {{args}}
    just build-gui {{args}}

# Build and install GUI
@install-gui:
    #!/bin/sh
    echo "\n=== Building keep_keeping in release mode ===\n"
    cd gui && cargo build --release
    echo "\n=== Installing keep_keeping ===\n"
    cp -f "target/release/keep_keeping_gui" "{{install_dir}}"
    echo "    Finished with exit code 0"
    echo "\n==> Finished, binary installed at '{{install_dir}}'"

# Build CLI
@build-cli +args='':
    #!/bin/sh
    cd cli && cargo build {{args}}

# Build GUI
@build-gui +args='':
    #!/bin/sh
    cd gui && cargo build {{args}}

# Run CLI
@run-cli +args='':
    target/debug/keep_keeping_cli {{args}}

# Run GUI
@run-gui +args='':
    target/debug/keep_keeping_gui {{args}}

# Build and run CLI
@build-run-cli +args='':
    just build-cli
    echo "\n=== Running keep_keeping_cli ===\n"
    just run-cli {{args}}

# Build and run GUI
@build-run-gui +args='':
    just build-gui
    echo "\n=== Running keep_keeping_gui ===\n"
    just run-gui {{args}}
# user_name        := env("USER")
# current_location := justfile()
current_dir      := justfile_directory()
module_name      := file_name(current_dir)
target_dir       := `cargo metadata --no-deps --format-version=1 | jq -r '.target_directory'`

default: build

build:
	cargo build
	RUST_BACKTRACE=1 cargo test
	cargo clippy

build-jvm:
    RUSTFLAGS="-C panic=unwind" cargo jvm build

cover-setup:
    cargo install cargo-llvm-cov
    rustup component add llvm-tools-preview

cover:
	cargo llvm-cov --all-features --workspace --html

cover-lcov:
	cargo llvm-cov --all-features --workspace --lcov --output-path {{target_dir}}/coverage/lcov.info

cover-text:
	cargo llvm-cov --all-features --workspace

install:
	cargo build --release
	cp {{target_dir}}/release/aifix ~/bin

fix:
    RUST_BACKTRACE=1 {{target_dir}}/debug/aifix -l rust -t fix_code -f {{current_dir}} -f {{current_dir}}/..

fixd:
    RUST_BACKTRACE=1 {{target_dir}}/debug/aifix -d -l rust -t fix_code -f {{current_dir}} -f {{current_dir}}/..

fixws:
    RUST_BACKTRACE=1 {{target_dir}}/debug/aifix -l rust -t fix_code -w ~/svn/_workspace -f {{current_dir}} -f {{current_dir}}/.. -f ~/svn/_workspace

doc_item:
    RUST_BACKTRACE=1 {{target_dir}}/debug/aifix -l rust -t write_item_doc -s src/aiagentloop.rs -f {{current_dir}} -f {{current_dir}}/..

review:
    RUST_BACKTRACE=1 {{target_dir}}/debug/aifix -l rust -t review_code -s src/aiagentloop.rs -f {{current_dir}} -f {{current_dir}}/..

doc_module:
    RUST_BACKTRACE=1 {{target_dir}}/debug/aifix -l rust -t write_module_doc -s src/aiagentloop.rs -f {{current_dir}} -f {{current_dir}}/..

test-jvm:
    java -cp $HOME/.local/share/cargo-jvm/rustc_codegen_jvm/runtime/build/libs/runtime-0.1.0.jar:{{target_dir}}/jvm-unknown-jvm/debug/collect_to_md.jar aifix.aifix

clean:
	@cargo clean -p {{module_name}}

clean-all:
	@cargo clean

targetlist:
    rustup target list

rpi:
    cargo build --target aarch64-unknown-linux-gnu



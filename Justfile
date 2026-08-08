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
    RUST_BACKTRACE=1 {{target_dir}}/debug/aifix -l rust -t write_item_doc -f {{current_dir}} -f {{current_dir}}/..

review:
    RUST_BACKTRACE=1 {{target_dir}}/debug/aifix -l rust -t review_code -f {{current_dir}} -f {{current_dir}}/..

doc_module:
    RUST_BACKTRACE=1 {{target_dir}}/debug/aifix -l rust -t write_module_doc -f {{current_dir}} -f {{current_dir}}/..

clean:
	@cargo clean -p {{module_name}}

clean-all:
	@cargo clean

targetlist:
    rustup target list

rpi:
    cargo build --target aarch64-unknown-linux-gnu

cover:
	CARGO_INCREMENTAL=0 RUSTFLAGS='-Cinstrument-coverage' LLVM_PROFILE_FILE='{{target_dir}}/coverage/cargo-test-%p-%m.profraw' cargo test
	grcov . --binary-path {{target_dir}}/debug/deps/ -s . -t html --branch --ignore-not-existing --ignore '../*' --ignore "/*" -o target/coverage/html
	firefox target/coverage/html/index.html


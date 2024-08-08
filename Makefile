TARGET_OPTION := $(if $(TARGET), --target $(TARGET))

build:
	cargo build --verbose

test:
	cargo test --verbose

release:
	cargo build --locked --release --verbose $(TARGET_OPTION)

coverage-lcov:
	cargo +nightly tarpaulin --verbose --all-features --workspace --timeout 120 --out Lcov --output-dir ./coverage

coverage-html:
	cargo +nightly tarpaulin --verbose --all-features --workspace --timeout 120 --out Html --output-dir ./coverage

clean:
	rm -rf coverage

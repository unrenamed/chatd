build:
	cargo build --verbose

release:
	cargo build --release --verbose

coverage-lcov:
	cargo +nightly tarpaulin --verbose --all-features --workspace --timeout 120 --out Lcov --output-dir ./coverage

coverage-html:
	cargo +nightly tarpaulin --verbose --all-features --workspace --timeout 120 --out Html --output-dir ./coverage

clean:
	rm -rf coverage

build:
	@echo "Building..."
	@wasm-pack build --target web --out-name index --release
	@pnpm format
	@echo "Done!"

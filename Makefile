.PHONY: run dev dev-fast build clean check

## SSR only — tanpa WASM hydration (cepat, halaman tidak interaktif)
run:
	cargo run

## Full dev dengan WASM hydration + hot reload (direkomendasikan)
dev:
	cargo leptos watch

## Production build
build:
	cargo leptos build --release

## Check / lint
check:
	cargo check --features ssr
	cargo clippy --features ssr

clean:
	cargo clean

# Development

Build and test the project through Docker using the `Justfile`:

```sh
docker compose build
docker compose run --rm build just build
docker compose run --rm build just test
```

Run the build, test, example, no_std, and WebAssembly pipeline with:

```sh
docker compose run --rm build just ci
```

Formatting and Clippy checks use stable Rust:

```sh
docker compose run --rm build just lint
```

If Docker or Just is unavailable, run the corresponding build and test steps
from the `Justfile` directly.

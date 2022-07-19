# Rust [cargo-chef](https://github.com/LukeMathWalker/cargo-chef) [dagger](https://github.com/dagger/dagger) action
The blog post [5x Faster Rust Docker Builds with cargo-chef](https://www.lpalmieri.com/posts/fast-rust-docker-builds/)[^1] explains how cargo-chef works and the problem it solves in detail. Here is a quick summary, but please go through the post for a more in-depth understanding. It's a great read anyway!

## Rust Docker builds are slow
The Rust compiler is slow and optimised builds (--release) can easily take 15/20 minutes on medium projects with several dependencies.
The Rust package manager cargo doesn't provide a mechanism to build only the dependencies[^2], so it's not straightforward to leverage the Docker layer caching.[^1]

## cargo-chef to the rescue
`cargo-chef` is a cargo sub-command to build just the dependencies of a Rust project and can be used to fully leverage Docker layer caching, therefore speeding up Docker builds.[^1]

The downside of using cargo-chef is that it brings a bit of complexity into the codebase.

The [Dockerfile](https://github.com/simonecarriero/rust-cargo-chef-dagger-action-example/blob/main/Dockerfile) would look like this:

```
FROM rust:1.62.0-slim as chef
RUN cargo install cargo-chef
WORKDIR app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin app

# We do not need the Rust toolchain to run the binary!
FROM debian:bullseye-slim AS runtime
WORKDIR app
COPY --from=builder /app/target/release/app /usr/local/bin
ENTRYPOINT ["/usr/local/bin/app"]
```

And here's how it works:

>We are using three stages: the first computes the recipe file, the second caches our dependencies and builds the binary, the third is our slim runtime environment.
>As long as your dependency tree does not change the recipe.json file will stay the same, therefore the outcome of cargo cargo chef cook --release --recipe-path recipe.json will be cached, massively speeding up your builds (up to 5x measured on some commercial projects).
>
>We are taking advantage of how Docker layer caching interacts with multi-stage builds: the COPY . . statement in the planner stage will invalidate the cache for the planner container, but it will not invalidate the cache for the builder container, as long as the checksum of the recipe.json returned by cargo chef prepare does not change.
>You can think of each stage as its own Docker image with its own caching - they only interact with each other when using the COPY --from statement.
>
>There is no rocket science at play here - you might argue that is just an elaborate way to perform the dirty workaround we talked about before. You would be right, to an extent.
>But ergonomics matters and cargo-chef offers a much more streamlined experience if you are newcomer looking for a quick and clean recipe to optimise your Docker build.
>
>-- 5x Faster Rust Docker Builds with cargo-chef, Luca Palmieri[^1]

Nothing extreme, it's a nice and clever solution. But still accidental complexity.

## cargochef.#Build dagger action

Dagger is a tool to build CI/CD pipelines based on the concept of actions, and actions are composable. 

The `cargochef.#Build` action wraps the complexity of using cargo-chef. You can compose it with your actions and keep that complexity out of your codebase.

The [dagger.cue](https://github.com/simonecarriero/rust-cargo-chef-dagger-action-example/blob/main/dagger.cue) would look like this: 

```
...
	
	actions: {

		cargochefBuild: cargochef.#Build & {
			projectDirectory: client.filesystem.".".read.contents
			rustDockerImage:  "rust:1.62.0-slim"
			workdir:          "/app"
			cargoBuildArgs: ["--release", "--bin", "app"]
		}

		runtime: docker.#Build & {
			steps: [
				docker.#Pull & {
					source: "debian:bullseye-slim"
				},
				docker.#Set & {
					config: workdir: "/app"
				},
				docker.#Copy & {
					contents: cargochefBuild.output.rootfs
					source:   "/app/target/release/app"
					dest:     "/usr/local/bin"
				},
				docker.#Set & {
					config: entrypoint: ["/usr/local/bin/app"]
				},
			]
		}

		build: cli.#Load & {
			image: runtime.output
			host:  client.network."unix:///var/run/docker.sock".connect
			tag:   "app"
		}
	}
```

As you can see, there's no reference to the first two stages but just the runtime environment.

## Install
```
dagger project update github.com/simonecarriero/rust-cargo-chef-dagger-action@v0.0.2
```

## Usage
See a [working example](https://github.com/simonecarriero/rust-cargo-chef-dagger-action-example).

[^1]: [5x Faster Rust Docker Builds with cargo-chef, Luca Palmieri](https://www.lpalmieri.com/posts/fast-rust-docker-builds/)
[^2]: [cargo build --dependencies-only #2644](https://github.com/rust-lang/cargo/issues/2644)

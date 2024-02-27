package cargochef

import (
    "dagger.io/dagger"
    "universe.dagger.io/docker"
)

#Build: {

	// Directory containing the Rust project to build
	projectDirectory: dagger.#FS

	// Rust Docker image to be used for building the project
	rustDockerImage: string

	// Workdir to be used in the build
	workdir: string

	// Arguments to the cargo build command
	cargoBuildArgs: [...string]

	let _workdir = workdir

	chef: docker.#Build & {
		steps: [
			docker.#Pull & {
				source: rustDockerImage
			},
			docker.#Run & {
				command: {
					name: "cargo"
					args: ["install", "cargo-chef"]
				}
			},
			docker.#Set & {
				config: workdir: _workdir
			},
		]
	}

	planner: docker.#Build & {
		steps: [
			docker.#Step & {
				output: chef.output
			},
			docker.#Copy & {
				contents: projectDirectory
				dest:     "."
			},
			docker.#Run & {
				command: {
					name: "cargo"
					args: ["chef", "prepare", "--recipe-path", "recipe.json"]
				}
			},
		]
	}

	builder: docker.#Build & {
		steps: [
			docker.#Step & {
				output: chef.output
			},
			docker.#Copy & {
				contents: planner.output.rootfs
				source:   "\(_workdir)/recipe.json"
				dest:     "recipe.json"
			},
			docker.#Run & {
				command: {
					name: "cargo"
					args: ["chef", "cook", "--release", "--recipe-path", "recipe.json"]
				}
			},
			docker.#Copy & {
				contents: projectDirectory
				source:   "."
				dest:     "."
			},
			docker.#Run & {
				command: {
					name: "cargo"
					args: ["build"] + cargoBuildArgs
				}
			},
		]
	}

	output: builder.output
}

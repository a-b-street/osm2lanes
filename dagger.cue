package main

import (
	"dagger.io/dagger"
	"github.com/simonecarriero/rust-cargo-chef-dagger-action/cargochef"
)

dagger.#Plan & {

	client: {
		filesystem: ".": read: contents: dagger.#FS
		network: "unix:///var/run/docker.sock": connect: dagger.#Socket
	}

	actions: {

		build: cargochef.#Build & {
			projectDirectory: client.filesystem.".".read.contents
			rustDockerImage: "rust:1.62.0"
			workdir: "/app"
		}

		_outfs: build.output.rootfs
	}
}

# checkin-embedded
A check-in system for embedded platforms like the Raspberry Pi that integrates with [HackGT/checkin2](https://github.com/HackGT/checkin2)

## Building
To build into a Docker container, build `Dockerfile`. `Dockerfile` inherits from the pre-built `Dockerfile.init` container which contains a cache of compiled libraries. If you need to update the base image (such as for updating Rust), build `Dockerfile.init` locally then `docker push` it to `hackgt/checkin-embedded-init`.
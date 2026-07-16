git_revision := `git rev-parse --short HEAD`
app_version := `awk -F'"' '/^\[package\]/{p=1} p && /^version *=/{print $2; exit}' Cargo.toml`
build_date := `date -u +%Y-%m-%dT%H:%M:%SZ`
dockerhub_namespace := env_var_or_default("DOCKERHUB_NAMESPACE", "sw1tch9789")
cli_image := dockerhub_namespace + "/colorfuls"
docs_image := dockerhub_namespace + "/colorfuls-docs"

container_runner := "docker"

test:
    cargo llvm-cov

docs:
    cd docs && hugo -d public

build: test
    cargo build --release

docker-login:
    {{ container_runner }} login docker.io

container-local: docs
    {{ container_runner }} build \
        --build-arg GIT_REVISION={{git_revision}} \
        --build-arg BUILD_DATE={{build_date}} \
        --build-arg VERSION={{app_version}} \
        -t {{ cli_image }}:latest -t {{ cli_image }}:{{ app_version }} \
        -f Containerfile \
        .
    {{ container_runner }} build \
        --build-arg GIT_REVISION={{git_revision}} \
        --build-arg BUILD_DATE={{build_date}} \
        --build-arg VERSION={{app_version}} \
        -t {{ docs_image }}:latest -t {{ docs_image }}:{{ app_version }} \
        -f Containerfile.docs \
        .

container: docs
    {{ container_runner }} buildx build --push \
        --platform linux/amd64,linux/arm64 \
        --build-arg GIT_REVISION={{git_revision}} \
        --build-arg BUILD_DATE={{build_date}} \
        --build-arg VERSION={{ app_version }} \
        -t {{ cli_image }}:latest -t {{ cli_image }}:{{ app_version }} \
        -f Containerfile \
        .
    {{ container_runner }} buildx build --push \
        --platform linux/amd64,linux/arm64 \
        --build-arg GIT_REVISION={{git_revision}} \
        --build-arg BUILD_DATE={{build_date}} \
        --build-arg VERSION={{ app_version }} \
        -t {{ docs_image }}:latest -t {{ docs_image }}:{{ app_version }} \
        -f Containerfile.docs \
        .

publish: docker-login container

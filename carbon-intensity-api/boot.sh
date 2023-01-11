# build and start the stack described by the compose.yaml file
if [ "$1" = "--build" ]; then
    cp .dockerignore ../.
    docker-compose build --no-cache
fi

docker-compose up -d
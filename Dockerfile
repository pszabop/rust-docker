# never use latest.  You will get nasty surprise rebuilds
FROM rust:1.83

## rust docker based build environment
## inherit from this to create an application specific build/deploy executable of your rust program 


## if you don't include lld link times will be a long nightmare
RUN apt-get update && apt-get install -y lld

## cmake is necessary to make cargo work
RUN apt-get update && apt-get install -y cmake

## If you you use any C or C++ librarys you will need these
RUN apt-get update && apt-get install -y clang


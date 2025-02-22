#################################
#     RTMP Server Dockerfile    #
#################################

# Builder
FROM rust:alpine AS builder

    ## Install OpenSSL
    RUN apk add libressl-dev musl-dev

    ## Copy files
    ADD . /root

    ## Compile
    WORKDIR /root
    ENV OPENSSL_NO_VENDOR=true
    RUN cargo build --release

# Runner
FROM alpine AS runner

    ## Install common libraries
    RUN apk add gcompat

    ## Copy binary
    COPY --from=builder /root/target/release/rtmp-server /usr/bin/rtmp-server

    # Expose ports
    EXPOSE 1935
    EXPOSE 443

    # Entry point
    ENTRYPOINT ["/usr/bin/rtmp-server"]

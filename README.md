# RTMP Server (Rust Implementation)

[![Rust](https://github.com/AgustinSRG/rtmp-server-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/AgustinSRG/rtmp-server-rs/actions/workflows/rust.yml)
[![License](https://img.shields.io/badge/license-MIT-blue.svg?style=flat)](./LICENSE)

This is a RTMP (Real Time Messaging Protocol) server for live streaming broadcasting, implemented in [Rust](https://www.rust-lang.org/).

This is a rewrite of [RTMP Server (Go Implementation)](https://github.com/AgustinSRG/rtmp-server).

This version's main goal is performance and efficiency:

- Lower memory usage
- Lower CPU usage
- Greater bit rate

## Compilation

In order to compile the project, run the following command:

```sh
cargo build --release
```

An executable binary will be generated in the `target/release` folder, called called `rtmp-server`, or `rtmp-server.exe` if you are using Windows.

## Docker image

You can find the docker image for this project available in Docker Hub: [https://hub.docker.com/r/asanrom/rtmp-server-rs](https://hub.docker.com/r/asanrom/rtmp-server-rs)

To pull it type:

```
docker pull asanrom/rtmp-server-rs
```

Example compose file:

```yml
version: "3.7"

services:
  rtmp_server:
    image: asanrom/rtmp-server-rs
    ports:
      - "1935:1935"
      #- '443:443'
    environment:
      # Configure it using env vars:
      - PLAY_ALLOWED_FROM=*
      - CONCURRENT_LIMIT_WHITELIST=*
      - REDIS_USE=NO
      - LOG_REQUESTS=YES
      - LOG_DEBUG=NO
      - GOP_CACHE_SIZE_MB=0
```

## Usage

In order to run the server you have to run the binary or the docker container. That will run the server in the port `1935`.

The server will accept RTMP connections with the following schema:

```
rtmp://{HOST}/{CHANNEL}/{KEY}
```

Note: Both `CHANNEL` and `KEY` are restricted to letters `a-z`, numbers `0-9`, dashes `-` and underscores `_`.

By default, it will accept any connections. If you need to restrict the access or customize the server in any way, you can use environment variables.

### RTMP play restrict

You probably only want external users to be able to publish to the RTMP server, since spectators probably receive the stream using other protocol, like HLS or MPEG-Dash.

In order to do that, set the `RTMP_PLAY_WHITELIST` to a list of allowed internet addresses split by commas. Example: `127.0.0.1,10.0.0.0/8`. You can set IPs, or subnets. It supports both IP version 4 and version 6.

### Event callback

In order to restrict the access and have control over who publishes, the RTMP server can send requests to a remote server with the information of certain events.

Set the `CALLBACK_URL` environment variable to the remote server that is going to handle those events:

- When an user wants to publish, to validate the streaming channel and key. (`start`)
- When a session is closed, meaning the live streaming has ended. (`stop`)

The events are sent as HTTP(S) **POST** requests to the given URL, with empty body, and with a header with name `rtmp-event`, containing the event data encoded as a **Base 64 JWT (JSON Web Token)**, signed using a secret you must provide using the `JWT_SECRET` environment variable.

The JWT is signed using the algorithm `HMAC_256`.

The JWT contains the following fields:

- Subject (`sub`) is `rtmp_event`.
- Event name (`event`) can be `start` or `stop`.
- Channel (`channel`) is the requested channel to publish.
- Key (`key`) is the given key to publish.
- Stream ID (`stream_id`) is the unique ID for the stream session, It is undefined for the `start` event, since is not known yet.
- Client IP (`client_ip`) is the client IP for logging purposes.

For the `start` event, the event handler server must return with status code **200**, and with a header with name `stream-id`, containing the unique identifier for the RTMP publishing session. If the server does not return with 200, the server will consider the key is invalid and it will close the connection with the client. You can use this to validate streaming keys.

### Redis

This server supports listening for commands using Redis Pub/Sub.

To configure it, set the following variables:

| Variable Name  | Description                                                         |
| -------------- | ------------------------------------------------------------------- |
| REDIS_USE      | Set it to `YES` in order to enable Redis.                           |
| REDIS_PORT     | Port to connect to Redis Pub/Sub. Default is `6379`                 |
| REDIS_HOST     | Host to connect to Redis Pub/Sub. Default is `127.0.0.1`            |
| REDIS_PASSWORD | Redis authentication password, if required.                         |
| REDIS_CHANNEL  | Redis channel to listen for commands. By default is `rtmp_commands` |
| REDIS_TLS      | Set it to `YES` in order to use TLS for the connection.             |

The commands have the following structure:

```
COMMAND>ARG_1|ARG2|...
```

Each command goes in a separate message.

List of commands:

- `kill-session>CHANNEL` - Closes any sessions for that specific channel.
- `close-stream>CHANNEL|STREAM_ID` - Closes specific connection.

These commands are meant to stop a streaming session once started, to enforce application-specific limits.

### Control server

In order to integrate this RTMP server with [tcp-video-streaming](https://github.com/AgustinSRG/tcp-video-streaming)'s control server, set `CONTROL_USE` to `YES`.

Also, configure the following variables:

| Variable Name    | Description                                                                                  |
| ---------------- | -------------------------------------------------------------------------------------------- |
| CONTROL_BASE_URL | Websocket URL to connect to the coordinator server. Example: `wss://10.0.0.0:8080/`          |
| CONTROL_SECRET   | Secret shared between the coordinator server and the RTMP server, in order to authenticate.  |
| EXTERNAL_IP      | IP address of the RTMP server in order to indicate it to the coordinator server              |
| EXTERNAL_PORT    | Listening port of the RTMP server in order to indicate it to the coordinator server          |
| EXTERNAL_SSL     | Set it to `YES` if the rest of components will need to use SSL to connect to the RTMP server |

Note: Enabling the control server will disable the callback request feature, replacing it with requests to the control server instead.

### TLS

If you want to use TLS, you have to set the following variables in order for it to work:

| Variable Name            | Description                                                                         |
| ------------------------ | ----------------------------------------------------------------------------------- |
| SSL_PORT                 | RTMPS (RTMP over TLS) listening port. Default is `443`                              |
| SSL_CERT                 | Path to SSL certificate (REQUIRED).                                                 |
| SSL_KEY                  | Path to SSL private key (REQUIRED).                                                 |
| SSL_CHECK_RELOAD_SECONDS | Number of seconds to check for changes in the certificate or key (for auto renewal) |

### Log options

Here is a list of options to customize log messages:

| Variable Name | Description                                                                                                    |
| ------------- | -------------------------------------------------------------------------------------------------------------- |
| LOG_ERROR     | Log error messages? Set to `YES` or `NO`. By default is `YES`                                                  |
| LOG_WARNING   | Log warning messages? Set to `YES` or `NO`. By default is `YES`                                                |
| LOG_INFO      | Log info messages? Set to `YES` or `NO`. By default is `YES`                                                   |
| LOG_REQUESTS  | Log incoming requests? Set to `YES` or `NO`. By default is `YES`. Note: requests are logged with info messages |
| LOG_DEBUG     | Log debug messages? Set to `YES` or `NO`. By default is `NO`                                                   |
| LOG_TRACE     | Log trace messages? Set to `YES` or `NO`. By default, it uses the value of `LOG_DEBUG`                         |

### DOS mitigation options

List of options made to mitigate DOS (Denial of Service) attacks.

| Variable Name                 | Description                                                                                                                        |
| ----------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| MAX_IP_CONCURRENT_CONNECTIONS | Max number of concurrent connections to accept from a single IP. By default is 4.                                                  |
| CONCURRENT_LIMIT_WHITELIST    | List of IP ranges not affected by the max number of concurrent connections limit. Split by commas. Example: `127.0.0.1,10.0.0.0/8` |

### Performance options

Lis of options related to performance.

| Variable Name     | Description                                                                                     |
| ----------------- | ----------------------------------------------------------------------------------------------- |
| RTMP_CHUNK_SIZE   | RTMP Chunk size in bytes. Default is `4096`                                                     |
| GOP_CACHE_SIZE_MB | Size limit in megabytes of packet cache. By default is `256`. Set it to `0` to disable cache    |
| MSG_BUFFER_SIZE   | Size of the message buffer. Default: `8`. Lower it to reduce memory usage at a cost of bit rate |

### More options

Here is a list with more options you can configure:

| Variable Name      | Description                                                                                           |
| ------------------ | ----------------------------------------------------------------------------------------------------- |
| RTMP_HOST          | RTMP host to add in the JWT as `rtmp_host` in order for the callback handler to know the origin host. |
| RTMP_PORT          | RTMP listening port. It will be added in the JWT as `rtmp_port`. Default is `1935`.                   |
| BIND_ADDRESS       | Bind address for RTMP and RTMPS. By default it binds to all network interfaces.                       |
| ID_MAX_LENGTH      | Max length for `CHANNEL` and `KEY`. By default is 128 characters                                      |
| CUSTOM_JWT_SUBJECT | Custom subject to use for tokens sent to the callback URL                                             |

## Testing

In order to run the unit tests, type:

```sh
cargo test
```

If you wish to test the server against a well-known client, you can use [FFmpeg](https://www.ffmpeg.org/) as the client.

In order to publish, run a command like this (replace the video file and the RTMP URL):

```sh
ffmpeg -re -stream_loop -1 -i video.mp4 -c:v copy -c:a copy -f flv rtmp://127.0.0.1/channel/key
```

In order to play, run a command like this:

```sh
ffplay rtmp://127.0.0.1/channel/key
```

## Benchmark

This repository also contains a [benchmark script](./benchmark) you can use to compare performances between versions.

## License

This project is under the [MIT license](./LICENSE).

# RTMP benchmark

This is a basic RTMP benchmark made for this project.

## Requirements

 - [NodeJS](https://nodejs.org/en/)
 - [FFmpeg](https://www.ffmpeg.org/)

## Usage

Copy the [.env.example](./.env.example) file into `.env` and modify the configuration variables required for this script to work.

Install dependencies with the following command:

```sh
npm install
```

Run the benchmark with the following command:

```sh
npm start
```

The benchmark will generate a CSV file with information about:

 - The process memory usage (bytes)
 - The process CPU usage (%)
 - The bit rate (bits per second)


You can then import this CSV file into a external tool in order to generate some charts.

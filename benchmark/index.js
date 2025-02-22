// RTMP benchmark
// Runs a RTMP test using FFMPEG
// Measures bitrate, memory usage and CPU usage

"use strict";

require("dotenv").config();

const Path = require("path");
const Fs = require("fs");
const pidUsage = require('pidusage');
const ChildProcess = require("child_process");

const logDebug = process.env.LOG_DEBUG === "YES";

const outputFile = process.env.OUTPUT_FILE || "benchmark_result.csv";

const outputFileStream = Fs.createWriteStream(outputFile);

outputFileStream.write("COUNT,DATE,BITRATE,CPU,MEM\n");

const ffmpegPath = process.env.FFMPEG_PATH || "/usr/bin/ffmpeg";
if (!ffmpegPath || !Fs.existsSync(ffmpegPath)) {
    console.error("FFmpeg not found: " + ffmpegPath);
    outputFileStream.close();
    process.exit(1);
}


const videoFile = process.env.VIDEO_FILE || "video.mp4";

if (!videoFile || !Fs.existsSync(videoFile)) {
    console.error("Video file not found: " + videoFile);
    outputFileStream.close();
    process.exit(1);
}

const rtmpUrl = process.env.RTMP_URL || "rtmp://127.0.0.1/channel/key";

const serverPid = parseInt(process.env.SERVER_PID || "0", 10) || 0;

const waitTime = (parseInt(process.env.WAIT_TIME_SECONDS || "1", 10) || 0) * 1000;

const duration = (parseInt(process.env.DURATION_SECONDS || "60", 10) || 60) * 1000;

const Status = {
    cpu: 0,
    mem: 0,
    bitrate: 0,
};

async function wait(ms) {
    return new Promise((resolve) => {
        setTimeout(resolve, ms);
    });
}

async function measureProcess(pid) {
    return new Promise((resolve) => {
        pidUsage(pid, (err, stats) => {
            if (err) {
                console.error(`Could not measure process ${pid}: ${err.message}`);
                return resolve({ cpu: 0, mem: 0 });
            }

            resolve({
                cpu: stats.cpu,
                mem: stats.memory,
            });
        });
    });
}

async function periodicallyMeasureProcess() {
    let lastUpdate = Date.now();

    while (1) {
        const stats = await measureProcess(serverPid);

        Status.cpu = stats.cpu;
        Status.mem = stats.mem;

        const now = Date.now();

        const timeSinceLastUpdate = now - lastUpdate;
        const waitTime = Math.max(0, 1000 - timeSinceLastUpdate);

        if (waitTime) {
            await wait(waitTime);
        }
    }
}

async function periodicallyMeasureProcess() {
    let lastUpdate = Date.now();

    while (1) {
        const stats = await measureProcess(serverPid);

        Status.cpu = stats.cpu;
        Status.mem = stats.mem;

        const now = Date.now();

        const timeSinceLastUpdate = now - lastUpdate;
        const waitTime = Math.max(0, 1000 - timeSinceLastUpdate);

        if (waitTime) {
            await wait(waitTime);
        }
    }
}

async function periodicallyWriteStats() {
    let count = 0;

    while(1) {
        const date = (new Date()).toISOString();

        const bitRateMb = Math.round((Status.bitrate / (1000 * 1000)) * 100) / 100;
        const memMb = Math.round((Status.mem / (1024 * 1024)) * 100) / 100;

        console.log(`[STATS] [${count}] [${date}] Bitrate=${bitRateMb} Mbit/s, CPU: ${Status.cpu}%, Memory usage: ${memMb} MB.`);

        outputFileStream.write(`${count},${JSON.stringify(date)},${Status.bitrate},${Status.cpu},${Status.mem}\n`);

        await wait(1000);
    }
}

async function main() {
    if (serverPid) {
        periodicallyMeasureProcess().catch(ex => {
            console.error(ex);
            outputFileStream.close();
            process.exit(1);
        });
    }

    periodicallyWriteStats().catch(ex => {
        console.error(ex);
        outputFileStream.close();
        process.exit(1);
    });

    // Wait before starting ffmpeg

    if (waitTime) {
        console.log("Waiting before starting FFmpeg...");
        await wait(waitTime);
    }


    // Start FFmpeg processes

    const readProcess = ChildProcess.spawn(ffmpegPath, ['-i', rtmpUrl, '-vcodec', 'copy', '-acodec', 'copy', '-f', 'mpegts', '-']);

    readProcess.stderr.on("data", data => {
        if (logDebug) {
            console.log("[FFMPEG R] " + data);
        }

        data = data + "";

        if (data.indexOf("bitrate=") >= 0) {
            const bitrateS = (data.split("bitrate=")[1].split("bits/s")[0] + "").toLowerCase();

            let bitrate = 0;

            if (bitrateS.endsWith("k")) {
                bitrate = Number(bitrateS.substring(0, bitrateS.length - 1)) * 1000;
            } else if (bitrateS.endsWith("m")) {
                bitrate = Number(bitrateS.substring(0, bitrateS.length - 1)) * 1000 * 1000;
            } else {
                bitrate = Number(bitrateS);
            }

            if (!isNaN(bitrate)) {
                Status.bitrate = bitrate;
            }
        }

    });

    readProcess.stdout.on("data", () => {}); // Ignore data

    const writeProcess = ChildProcess.spawn(ffmpegPath, ["-re", "-stream_loop", "-1", '-i', Path.resolve(__dirname, videoFile), '-vcodec', 'copy', '-acodec', 'copy', '-f', 'flv', rtmpUrl]);

    writeProcess.stderr.on("data", data => {
        if (logDebug) {
            console.log("[FFMPEG W] " + data);
        }
    });

    // Wait to end

    await wait(duration);
}

main().then(() => {
    console.log("Benchmark completed!");
    console.log("You can find the results in the following file: " + outputFile);
    outputFileStream.close();
    process.exit(0);
}).catch(ex => {
    console.error(ex);
    outputFileStream.close();
    process.exit(1);
});



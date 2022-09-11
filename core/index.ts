// This module should be the client that connects via IPC to the core. It will be available
// to the preload script and the main window.

const net = require("net");

const client = new net.Socket();
client.connect(9001, "127.0.0.1", function () {
  client.write("ping");
});

client.on("data", function (data: any) {
  console.log("[CLIENT] Received: " + data);
  if (data.toString() === "pong") {
    client.write("ping");
  }
});

client.on("close", function () {
  console.log("Connection closed");
});

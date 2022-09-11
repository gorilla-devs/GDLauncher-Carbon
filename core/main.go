package main

import (
	"bytes"
	"fmt"
	"log"
	"net"
	"os"
)

const (
	HOST = "localhost"
	PORT = "9001"
	TYPE = "tcp"
)

func main() {
	listen, err := net.Listen(TYPE, HOST+":"+PORT)
	if err != nil {
		log.Fatal(err)
		os.Exit(1)
	}
	// close listener
	defer listen.Close()

	conn, err := listen.Accept()

	if err != nil {
		log.Fatal(err)
		os.Exit(1)
	}

	handleIncomingRequest(conn)
}
func handleIncomingRequest(conn net.Conn) {

	for {
		buffer := make([]byte, 1024)
		_, err := conn.Read(buffer)
		if err != nil {
			log.Fatal(err)
		}

		if string(bytes.Trim(buffer, "\x00")) == "ping" {
			fmt.Println("It's ping ping")
			conn.Write([]byte("pong"))
		}

	}

	// respond
	// time := time.Now().Format("Monday, 02-Jan-06 15:04:05 MST")
	// conn.Write([]byte("Hi back!\n"))
	// conn.Write([]byte(time))
}

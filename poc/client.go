package main

import (
	"bufio"
	"fmt"
	"net"
	"time" // <-- ADD THIS
)

func send(conn net.Conn, cmd string) {
	fmt.Fprintf(conn, "%s\n", cmd)
	reply, _ := bufio.NewReader(conn).ReadString('\n')
	fmt.Printf("> %s -> %s", cmd, reply)
}

func main() {
	conn, err := net.Dial("tcp", "127.0.0.1:11223")
	if err != nil {
		panic(err)
	}
	defer conn.Close()

	send(conn, "SET username 5 john_doe")
	send(conn, "GET username")
	send(conn, "STATS")

	fmt.Println("Waiting 6 seconds for expiration...")
	time.Sleep(6 * time.Second) // <-- FIXED

	send(conn, "GET username")
	send(conn, "STATS")
}

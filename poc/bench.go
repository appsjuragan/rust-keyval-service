package main

import (
	"bufio"
	"fmt"
	"net"
	"os"
	"strconv"
	"sync"
	"sync/atomic"
	"time"
)

const (
	serverAddr = "127.0.0.1:11223"
)

func worker(id int, duration time.Duration, ops *uint64, wg *sync.WaitGroup) {
	defer wg.Done()

	conn, err := net.Dial("tcp", serverAddr)
	if err != nil {
		fmt.Println("Dial error:", err)
		return
	}
	defer conn.Close()

	reader := bufio.NewReader(conn)
	end := time.Now().Add(duration)

	for time.Now().Before(end) {
		// Example benchmark: SET and GET
		fmt.Fprintf(conn, "SET key%d 10 value%d\n", id, id)
		_, _ = reader.ReadString('\n')

		fmt.Fprintf(conn, "GET key%d\n", id)
		_, _ = reader.ReadString('\n')

		atomic.AddUint64(ops, 2)
	}
}

func main() {
	if len(os.Args) < 3 {
		fmt.Println("Usage: bench <threads> <seconds>")
		return
	}

	threads, _ := strconv.Atoi(os.Args[1])
	seconds, _ := strconv.Atoi(os.Args[2])

	var ops uint64
	var wg sync.WaitGroup

	fmt.Printf("🔥 Starting benchmark: %d threads, %d seconds\n", threads, seconds)

	start := time.Now()

	for i := 0; i < threads; i++ {
		wg.Add(1)
		go worker(i, time.Duration(seconds)*time.Second, &ops, &wg)
	}

	wg.Wait()
	elapsed := time.Since(start)

	totalOps := atomic.LoadUint64(&ops)
	qps := float64(totalOps) / elapsed.Seconds()

	fmt.Println("\n====== Benchmark Results ======")
	fmt.Printf("Total Ops : %d\n", totalOps)
	fmt.Printf("Duration  : %.2fs\n", elapsed.Seconds())
	fmt.Printf("Throughput: %.2f ops/sec\n", qps)
	fmt.Println("================================")
}

package main

import (
	"bufio"
	"fmt"
	"math/rand"
	"net"
	"os"
	"strconv"
	"sync"
	"sync/atomic"
	"time"
)

const serverAddr = "127.0.0.1:11223"

func randKey(n int) string {
	return fmt.Sprintf("key_%d", rand.Intn(n))
}

func worker(id int, cacheKeys int, duration time.Duration, ops *uint64, wg *sync.WaitGroup) {
	defer wg.Done()

	conn, err := net.Dial("tcp", serverAddr)
	if err != nil {
		fmt.Println("dial error:", err)
		return
	}
	defer conn.Close()

	r := bufio.NewReader(conn)
	end := time.Now().Add(duration)

	for time.Now().Before(end) {
		// 30% hit, 70% miss
		if rand.Intn(100) < 30 {
			// hit key
			key := randKey(cacheKeys)
			fmt.Fprintf(conn, "GET %s\n", key)
			_, _ = r.ReadString('\n')
			atomic.AddUint64(ops, 1)
		} else {
			// miss key (use bigger random space)
			key := randKey(cacheKeys * 10)
			fmt.Fprintf(conn, "GET %s\n", key)
			_, _ = r.ReadString('\n')
			atomic.AddUint64(ops, 1)
		}

		// occasional writes (help populate)
		if rand.Intn(100) < 5 {
			key := randKey(cacheKeys)
			val := fmt.Sprintf("value_%d", rand.Intn(999999))
			fmt.Fprintf(conn, "SET %s 30 %s\n", key, val)
			_, _ = r.ReadString('\n')
			atomic.AddUint64(ops, 1)
		}
	}
}

func main() {
	if len(os.Args) < 4 {
		fmt.Println("Usage: bench <threads> <seconds> <cache_size>")
		return
	}

	threads, _ := strconv.Atoi(os.Args[1])
	seconds, _ := strconv.Atoi(os.Args[2])
	cacheSize, _ := strconv.Atoi(os.Args[3])

	// Seed RNG
	rand.Seed(time.Now().UnixNano())

	fmt.Printf("🔥 Starting benchmark\n")
	fmt.Printf("Threads      : %d\n", threads)
	fmt.Printf("Seconds      : %d\n", seconds)
	fmt.Printf("Cache Keys   : %d\n", cacheSize)
	fmt.Printf("Hit Ratio    : ~30%%\n\n")

	var wg sync.WaitGroup
	var ops uint64
	duration := time.Duration(seconds) * time.Second

	// warm-up: populate cache
	fmt.Println("🔧 Warming cache...")
	func() {
		conn, _ := net.Dial("tcp", serverAddr)
		r := bufio.NewReader(conn)
		for i := 0; i < cacheSize; i++ {
			fmt.Fprintf(conn, "SET key_%d 30 warmvalue\n", i)
			r.ReadString('\n')
		}
		conn.Close()
	}()
	fmt.Println("Warm-up done.\n")

	start := time.Now()

	for i := 0; i < threads; i++ {
		wg.Add(1)
		go worker(i, cacheSize, duration, &ops, &wg)
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

package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"strconv"
	"sync"
	"sync/atomic"
	"time"
)

const (
	leaderAddr = "http://127.0.0.1:8081" // Node 1 (Leader)
)

type SetRequest struct {
	Value string `json:"value"`
}

func worker(id int, duration time.Duration, ops *uint64, wg *sync.WaitGroup) {
	defer wg.Done()

	client := &http.Client{
		Transport: &http.Transport{
			MaxIdleConns:        100,
			MaxIdleConnsPerHost: 100,
			IdleConnTimeout:     90 * time.Second,
		},
	}
	
	end := time.Now().Add(duration)
	payload, _ := json.Marshal(SetRequest{Value: "bench-value"})

	for time.Now().Before(end) {
		// SET request
		req, _ := http.NewRequest("POST", fmt.Sprintf("%s/key-%d", leaderAddr, id), bytes.NewBuffer(payload))
		req.Header.Set("Content-Type", "application/json")
		resp, err := client.Do(req)
		
		if err == nil {
			resp.Body.Close()
			atomic.AddUint64(ops, 1)
		}

		// GET request
		resp, err = client.Get(fmt.Sprintf("%s/key-%d", leaderAddr, id))
		if err == nil {
			resp.Body.Close()
			atomic.AddUint64(ops, 1)
		}
	}
}

func main() {
	if len(os.Args) < 3 {
		fmt.Println("Usage: bench_rest <threads> <seconds>")
		return
	}

	threads, _ := strconv.Atoi(os.Args[1])
	seconds, _ := strconv.Atoi(os.Args[2])

	var ops uint64
	var wg sync.WaitGroup

	fmt.Printf("🚀 Starting High-Throughput REST Benchmark: %d threads, %d seconds\n", threads, seconds)
	fmt.Printf("Target: %s\n", leaderAddr)

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

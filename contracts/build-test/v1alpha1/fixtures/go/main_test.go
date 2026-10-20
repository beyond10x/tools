package main

import "testing"

func TestGreeting(t *testing.T) {
	if got := greeting(); got != "hello from the go fixture" {
		t.Fatalf("greeting() = %q", got)
	}
}

func BenchmarkGreeting(b *testing.B) {
	for i := 0; i < b.N; i++ {
		_ = greeting()
	}
}

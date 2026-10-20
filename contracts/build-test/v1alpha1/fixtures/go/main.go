package main

import "fmt"

func greeting() string {
	return "hello from the go fixture"
}

func main() {
	fmt.Println(greeting())
}

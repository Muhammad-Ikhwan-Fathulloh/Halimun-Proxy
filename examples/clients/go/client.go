package main

import (
	"bytes"
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
)

// Configuration
const (
	aesKeyHex = "0000000000000000000000000000000000000000000000000000000000000000" // 32 Bytes
	proxyURL  = "http://localhost:80/proxy/1/abc/dummy/xyz/qwe/123"
	alphabet  = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567"
)

func main() {
	fmt.Println("Preparing Secure Payload...")

	payload := map[string]interface{}{
		"email":  "ikhwan@example.com",
		"action": "login",
	}

	encryptedBody, err := encryptPayload(payload)
	if err != nil {
		panic(err)
	}
	fmt.Printf("Encrypted Output: %s\n\n", encryptedBody)

	fmt.Println("Sending to Halimun Proxy...")
	
	data := url.Values{}
	data.Set("x", encryptedBody)

	resp, err := http.PostForm(proxyURL, data)
	if err != nil {
		fmt.Printf("Failed to reach proxy: %v\n", err)
		return
	}
	defer resp.Body.Close()

	bodyBytes, _ := io.ReadAll(resp.Body)
	fmt.Printf("Response Status: %d\n", resp.StatusCode)
	fmt.Printf("Response Body: %s\n", string(bodyBytes))
}

func encryptPayload(data map[string]interface{}) (string, error) {
	// 1. Setup Key & IV
	aesKey, _ := hex.DecodeString(aesKeyHex)
	block, err := aes.NewCipher(aesKey)
	if err != nil {
		return "", err
	}

	iv := make([]byte, aes.BlockSize)
	if _, err := io.ReadFull(rand.Reader, iv); err != nil {
		return "", err
	}

	// 2. PKCS7 Padding & Encrypt
	jsonBytes, _ := json.Marshal(data)
	paddedData := pkcs7Pad(jsonBytes, block.BlockSize())
	
	ciphertext := make([]byte, len(paddedData))
	mode := cipher.NewCBCEncrypter(block, iv)
	mode.CryptBlocks(ciphertext, paddedData)

	// 3. Prepend IV
	combined := append(iv, ciphertext...)

	// 4. Obfuscate XOR
	for i := range combined {
		combined[i] = combined[i] ^ 0xAC
	}

	// 5. Custom Base32
	return base32Encode(combined), nil
}

func pkcs7Pad(data []byte, blockSize int) []byte {
	padding := blockSize - len(data)%blockSize
	padtext := bytes.Repeat([]byte{byte(padding)}, padding)
	return append(data, padtext...)
}

func base32Encode(data []byte) string {
	bits := 0
	value := 0
	output := ""

	for _, b := range data {
		value = (value << 8) | int(b)
		bits += 8
		for bits >= 5 {
			output += string(alphabet[(value>>(bits-5))&31])
			bits -= 5
		}
	}
	if bits > 0 {
		output += string(alphabet[(value<<(5-bits))&31])
	}
	return output
}

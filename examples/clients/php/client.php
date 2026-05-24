<?php

// Configuration
$AES_KEY = hex2bin('0000000000000000000000000000000000000000000000000000000000000000'); // 32 Bytes
$PROXY_URL = 'http://localhost:80/proxy/1/abc/dummy/xyz/qwe/123';

// 1. Obfuscation XOR (rotate 0xAC)
function obfuscate($binaryString) {
    $len = strlen($binaryString);
    for ($i = 0; $i < $len; $i++) {
        $binaryString[$i] = chr(ord($binaryString[$i]) ^ 0xAC);
    }
    return $binaryString;
}

// 2. Custom Base32 Encoding (No Padding)
function base32Encode($binaryString) {
    $alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
    $len = strlen($binaryString);
    $bits = 0;
    $value = 0;
    $output = '';

    for ($i = 0; $i < $len; $i++) {
        $value = ($value << 8) | ord($binaryString[$i]);
        $bits += 8;
        while ($bits >= 5) {
            $output .= $alphabet[($value >> ($bits - 5)) & 31];
            $bits -= 5;
        }
    }
    if ($bits > 0) {
        $output .= $alphabet[($value << (5 - $bits)) & 31];
    }
    return $output;
}

// 3. Encrypt Payload
function encryptPayload($data, $apiKey) {
    $iv = random_bytes(16);
    $jsonObj = json_encode($data);
    
    // Encrypt
    $encrypted = openssl_encrypt($jsonObj, 'aes-256-cbc', $apiKey, OPENSSL_RAW_DATA, $iv);
    
    // Prepend IV
    $combined = $iv . $encrypted;
    
    // Obfuscate
    $obfuscated = obfuscate($combined);
    
    // Return Base32
    return base32Encode($obfuscated);
}

// ========================
// Usage Example
// ========================
echo "Preparing Secure Payload...\n";
$payload = [
    "email" => "ikhwan@example.com",
    "action" => "login",
    "timestamp" => time() * 1000
];

$encryptedBody = encryptPayload($payload, $AES_KEY);
echo "Encrypted Output: " . $encryptedBody . "\n\n";

echo "Sending to Halimun Proxy...\n";
$ch = curl_init($PROXY_URL);
curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);
curl_setopt($ch, CURLOPT_POST, true);
curl_setopt($ch, CURLOPT_POSTFIELDS, http_build_query(['x' => $encryptedBody]));

$response = curl_exec($ch);
$status = curl_getinfo($ch, CURLINFO_HTTP_CODE);
curl_close($ch);

echo "Response Status: " . $status . "\n";
echo "Response Body: " . $response . "\n";

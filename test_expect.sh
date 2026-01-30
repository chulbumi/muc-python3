#!/usr/bin/expect -f
set timeout 3

# Test Python MUD
spawn nc localhost 9900
expect "무림존함"
send "테스터\r"
expect {
    "암호" { send "test1234\r" }
    "새" { send "test\r" }
}
expect "보기"
send "보기\r"
expect "체력" { puts "\n=== Python MUD 보기 response ===" }
sleep 1
close

# Test Rust MUD
spawn nc localhost 9990
expect "무림존함"
send "테스터\r"
expect {
    "암호" { send "test1234\r" }
    "새" { send "test\r" }
}
expect "보기"
send "보기\r"
expect "체력" { puts "\n=== Rust MUD 보기 response ===" }
sleep 1
close

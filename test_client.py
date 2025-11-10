#!/usr/bin/env python3
import socket
import msgpack

def test_syncore():
    # Connect to Unix socket
    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.connect('/tmp/syncore.sock')
    
    # Test MemoryStore
    store_msg = {
        'tool': 'MemoryStore',
        'args': msgpack.packb(('test_key', 'test_value'))
    }
    
    msg_bytes = msgpack.packb(store_msg)
    if msg_bytes is not None:
        sock.sendall(msg_bytes)
    
    response = sock.recv(1024)
    result = msgpack.unpackb(response)
    print(f"Store result: {result}")
    
    # Test MemoryQuery
    sock.close()
    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.connect('/tmp/syncore.sock')
    
    query_msg = {
        'tool': 'MemoryQuery', 
        'args': msgpack.packb('test_key')
    }
    
    msg_bytes = msgpack.packb(query_msg)
    if msg_bytes is not None:
        sock.sendall(msg_bytes)
    
    response = sock.recv(1024)
    result = msgpack.unpackb(response)
    print(f"Query result: {result}")
    
    sock.close()

if __name__ == '__main__':
    test_syncore()
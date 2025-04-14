import socket
import json
import random
import time

def test_tcp_registration():
    # Generate random values for testing
    random_username = f"user_{random.randint(1000, 9999)}"
    random_pip = f"203.0.{random.randint(1, 255)}.{random.randint(1, 255)}"
    random_ip = f"192.168.{random.randint(1, 255)}.{random.randint(1, 255)}"
    random_port = random.randint(10000, 65000)
    
    # Create the registration message
    register_msg = {
        "type": "register",
        "username": random_username,
        "pip": random_pip,
        "ip": random_ip,
        "port": random_port
    }
    
    print(f"Attempting to register with these values:")
    print(f"Username: {random_username}")
    print(f"Public IP: {random_pip}")
    print(f"Local IP: {random_ip}")
    print(f"Port: {random_port}")
    
    try:
        # Create a TCP socket
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        
        # Set a timeout of 5 seconds
        s.settimeout(5)
        
        # Connect to the server's TCP port
        server_address = '54.66.23.75'  # Change this to your server address
        tcp_port = 8765
        
        print(f"\nConnecting to {server_address}:{tcp_port}...")
        s.connect((server_address, tcp_port))
        
        # Send the registration message
        message = json.dumps(register_msg)
        print(f"\nSending message: {message}")
        s.send(message.encode())
        
        # Wait for a response
        print("\nWaiting for response...")
        response = s.recv(1024).decode()
        print(f"Server response: {response}")
        
        # Try to parse the response as JSON
        try:
            parsed_response = json.loads(response)
            print("\nParsed response:")
            for key, value in parsed_response.items():
                print(f"  {key}: {value}")
        except json.JSONDecodeError:
            print("Could not parse response as JSON")
        
    except socket.timeout:
        print("Connection timed out. Make sure the server is running and accepting connections.")
    except ConnectionRefusedError:
        print("Connection refused. Make sure the server is running on the specified address and port.")
    except Exception as e:
        print(f"Error: {str(e)}")
    finally:
        # Close the connection
        s.close()
        print("\nConnection closed")

if __name__ == "__main__":
    test_tcp_registration()
    
    # Optionally, test again after a delay
    # time.sleep(2)
    # test_tcp_registration()
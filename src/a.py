import socket
import threading
import time

MY_LISTEN_PORT = 5544  # Choose a port (must match FRIEND_PUBLIC_PORT in Script 1)
FRIEND_PUBLIC_IP = "49.207.49.159"  # Replace with your public IP
FRIEND_PUBLIC_PORT = 5815  # Replace with your chosen port (must match MY_LISTEN_PORT in Script 1)

def receive_data():
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.bind(('', MY_LISTEN_PORT))
    print(f"Listening for incoming data on port {MY_LISTEN_PORT}")
    while True:
        try:
            data, addr = sock.recvfrom(1024)
            print(f"Received '{data.decode()}' from {addr}")
        except Exception as e:
            print(f"Error receiving data: {e}")

def send_punch():
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    print(f"Attempting to punch hole to {FRIEND_PUBLIC_IP}:{FRIEND_PUBLIC_PORT}")
    for _ in range(5):  # Send a few packets
        sock.sendto(b"PUNCH_HOLE", (FRIEND_PUBLIC_IP, FRIEND_PUBLIC_PORT))
        print("Sent punch packet")
        time.sleep(0.5)
    print("Punching attempts finished.")

if __name__ == "__main__":
    receive_thread = threading.Thread(target=receive_data)
    receive_thread.daemon = True
    receive_thread.start()

    input("Press Enter to start sending punch packets...")
    send_punch()

    # After punching, try sending some data back
    time.sleep(2)  # Give some time
    send_socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    message = "Hello from your friend's machine!"
    send_socket.sendto(message.encode(), (FRIEND_PUBLIC_IP, FRIEND_PUBLIC_PORT))
    print(f"Sent: '{message}' to {FRIEND_PUBLIC_IP}:{FRIEND_PUBLIC_PORT}")

    while True:
        time.sleep(1) # Keep the main thread alive
# server.py
import socket
import json

users = {}
userid = 0

def handle_message(message):
    global userid
    global users
    msg_type = message.get("type")

    if msg_type == "register":
        users[userid] = message
        print("Received Register:", message.get("username"))
        print(users)
        userid += 1
    elif msg_type == "get_users":
        print("Status update:", message.get("status"))
        return json.dumps(users) + "\n"
    elif msg_type == "command":
        print("Run command:", message.get("cmd"))
    else:
        print("Unknown Message type:", msg_type)

def start_server(host='0.0.0.0', port=8765):
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as server:
        server.bind((host, port))
        server.listen()
        print(f"Listening on {host}:{port}")

        while True:
            conn, addr = server.accept()
            with conn:
                print(f"Connected by {addr}")
                data = conn.recv(8192).decode()
                try:
                    message = json.loads(data)
                    response = handle_message(message)
                    if response:
                        conn.sendall(response.encode())
                except json.JSONDecodeError:
                    print("Invalid JSON received.")

start_server()
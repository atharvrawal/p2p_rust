import websockets
import asyncio
import json
import socket
from concurrent.futures import ThreadPoolExecutor

users = {}
userid = 0
peers = {}
relay_sessions = {}  # maps websocket -> peer websocket

def get_username_by_ws(ws):
    for username, info in peers.items():
        if info["websocket"] == ws:
            return username
    return None

async def signalling_handler(websocket):
    try:
        async for message in websocket:
            print("\n📡 Active users:", list(peers.keys()))

            if isinstance(message, bytes):
                # Relay binary message if in a relay session
                if websocket in relay_sessions:
                    peer_ws = relay_sessions[websocket]
                    await peer_ws.send(message)
                else:
                    print("⚠️ Received binary message but not in relay")
                continue

            data = json.loads(message)

            msg_type = data.get("type")

            if msg_type == "register":
                peers[data["username"]] = {
                    "ipv4_ip": data["ipv4_ip"],
                    "ipv4_port": data["ipv4_port"],
                    "ipv6_ip": data["ipv6_ip"],
                    "ipv6_port": data["ipv6_port"],
                    "password" : data["password"],
                    "websocket": websocket
                }
                await websocket.send(json.dumps({"status": "registered"}))
                print(f"✅ Registered: {data['username']}")

            if msg_type == "getusers":
                peers_cleaned = {
                    username : {
                        key:value
                        for key,value in info.items()
                        if key != "websocket"
                    }
                    for username, info in peers.items()
                }
                await websocket.send(json.dumps(peers_cleaned))

            if msg_type == "initiate_relay":
                target = data.get("target")
                sender = get_username_by_ws(websocket)

                if not target or target not in peers:
                    await websocket.send(json.dumps({"error": "Target user not found"}))
                else:
                    target_ws = peers[target]["websocket"]

                    # Register the session (both directions)
                    relay_sessions[websocket] = target_ws
                    relay_sessions[target_ws] = websocket

                    response = {
                        "status": "relay_initiated",
                        "type": "relay_initiated",
                        "target": target,
                        "initiator": sender,
                    }

                    await websocket.send(json.dumps(response))      # Tell sender
                    await target_ws.send(json.dumps(response))      # Tell receiver

                    print(f"🔄 Relay started between {sender} and {target}")

            elif msg_type == "relay_control" and data.get("action") == "end":
                peer_ws = relay_sessions.pop(websocket, None)
                if peer_ws:
                    await peer_ws.send(json.dumps({
                        "type": "relay_control",
                        "action": "end"
                    }))
                    relay_sessions.pop(peer_ws, None)
                    print("❌ Relay session ended.")

            elif msg_type == "peer_information":
                target = data.get("target")
                if target in peers:
                    info = peers[target]
                    await websocket.send(json.dumps({
                        "type": "peer_info",
                        "pip": info["pip"],
                        "ip": info["ip"],
                        "port": info["port"]
                    }))
                else:
                    await websocket.send(json.dumps({"error": "User not found"}))

            else:
                # Relay all other JSON messages if in a session
                if websocket in relay_sessions:
                    peer_ws = relay_sessions[websocket]
                    await peer_ws.send(message)
                else:
                    await websocket.send(json.dumps({"error": "Unknown or invalid request type"}))

    except websockets.exceptions.ConnectionClosed:
        print(f"🔌 Client disconnected.")
        # Clean up on disconnect
        username = get_username_by_ws(websocket)
        if username:
            print(f"Removing user: {username}")
            del peers[username]

        peer_ws = relay_sessions.pop(websocket, None)
        if peer_ws:
            relay_sessions.pop(peer_ws, None)
            try:
                await peer_ws.send(json.dumps({
                    "type": "relay_control",
                    "action": "end"
                }))
            except:
                pass

def handle_message(message):
    global userid
    global users
    msg_type = message.get("type")

    if msg_type == "register":
        users[userid] = message
        print("Received Register:", message.get("username"))
        print(users)
        userid += 1
        return json.dumps({"status": "registered", "userid": userid-1}) + "\n"
    elif msg_type == "get_users":
        print("Status update:", message.get("status"))
        return json.dumps(users) + "\n"
    elif msg_type == "command":
        print("Run command:", message.get("cmd"))
        return json.dumps({"status": "command_received"}) + "\n"
    elif msg_type == "request_peer":
        usernames = {}
        for uid, user_data in users.items():
            usernames[uid] = user_data.get("username")
        return json.dumps(usernames) + "\n"
    elif msg_type == "peer_information":
        target_id = message.get("target")
        if target_id is not None and str(target_id) in str(users.keys()):
            target_id = int(target_id)
            if target_id in users:
                info = users[target_id]
                return json.dumps({
                    "type": "peer_info",
                    "pip": info.get("pip"),
                    "ip": info.get("ip"),
                    "port": info.get("port")
                }) + "\n"
        return json.dumps({"error": "User not found"}) + "\n"
    else:
        print("Unknown Message type:", msg_type)
        return json.dumps({"error": "Unknown or invalid request type"}) + "\n"

async def handle_tcp_client(reader, writer):
    try:
        data = await reader.read(8192)
        message = data.decode()
        addr = writer.get_extra_info('peername')
        print(f"Received TCP request from {addr}: {message}")
        
        try:
            json_data = json.loads(message)
            response = handle_message(json_data)
            if response:
                writer.write(response.encode())
                await writer.drain()
                print(f"Sent TCP response: {response.strip()}")
            else:
                writer.write(json.dumps({"error": "No response generated"}).encode())
                await writer.drain()
        except json.JSONDecodeError:
            print("Invalid JSON received.")
            writer.write(json.dumps({"error": "Invalid JSON"}).encode())
            await writer.drain()
    except Exception as e:
        print(f"TCP error: {str(e)}")
    finally:
        writer.close()
        await writer.wait_closed()
        print(f"TCP connection with {addr} closed")

async def start_tcp_server(host='0.0.0.0', port=8765):
    server = await asyncio.start_server(
        handle_tcp_client,
        host,
        port
    )
    print(f"TCP server listening on {host}:{port}")
    async with server:
        await server.serve_forever()

async def main():
    # Start the WebSocket server
    websocket_server = await websockets.serve(
        signalling_handler,
        "0.0.0.0",
        9876,
        max_size=None  # So you can send large files
    )
    print(f"🚀 Signaling server running at 0.0.0.0:9876")

    # Start the TCP server
    tcp_server_task = asyncio.create_task(start_tcp_server())

    # Run both servers
    await asyncio.gather(
        websocket_server.wait_closed(),
        tcp_server_task
    )

if __name__ == "__main__":
    asyncio.run(main())
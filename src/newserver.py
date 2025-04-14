import asyncio
import socket
import json
import websockets

users = {}
userid = 0
peers = {}
relay_sessions = {}

def handle_tcp_message(message):
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

async def start_tcp_server(host='0.0.0.0', port=8765):
    server = await asyncio.start_server(tcp_server_loop, host, port)
    print(f"📡 TCP server running on {host}:{port}")
    async with server:
        await server.serve_forever()

async def tcp_server_loop(reader, writer):
    addr = writer.get_extra_info('peername')
    print(f"Connected by {addr}")
    data = await reader.read(8192)
    try:
        message = json.loads(data.decode())
        response = handle_tcp_message(message)
        if response:
            writer.write(response.encode())
            await writer.drain()
    except json.JSONDecodeError:
        print("Invalid JSON received.")
    writer.close()
    await writer.wait_closed()

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
                if websocket in relay_sessions:
                    peer_ws = relay_sessions[websocket]
                    await peer_ws.send(message)
                else:
                    print("⚠ Received binary message but not in relay")
                continue

            data = json.loads(message)
            msg_type = data.get("type")

            if msg_type == "register":
                peers[data["username"]] = {
                    "pip": data["pip"],
                    "ip": data["ip"],
                    "port": data["port"],
                    "websocket": websocket
                }
                await websocket.send(json.dumps({"status": "registered"}))
                print(f"✅ Registered: {data['username']}")

            elif msg_type == "request_peer":
                usernames = list(peers.keys())
                await websocket.send(json.dumps(usernames))

            elif msg_type == "initiate_relay":
                target = data.get("target")
                sender = get_username_by_ws(websocket)

                if not target or target not in peers:
                    await websocket.send(json.dumps({"error": "Target user not found"}))
                else:
                    target_ws = peers[target]["websocket"]
                    relay_sessions[websocket] = target_ws
                    relay_sessions[target_ws] = websocket
                    response = {
                        "status": "relay_initiated",
                        "type": "relay_initiated",
                        "target": target,
                        "initiator": sender,
                    }
                    await websocket.send(json.dumps(response))
                    await target_ws.send(json.dumps(response))
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
                if websocket in relay_sessions:
                    peer_ws = relay_sessions[websocket]
                    await peer_ws.send(message)
                else:
                    await websocket.send(json.dumps({"error": "Unknown or invalid request type"}))

    except websockets.exceptions.ConnectionClosed:
        print(f"🔌 Client disconnected.")
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

async def main():
    websocket_server = websockets.serve(signalling_handler, '0.0.0.0', 9876)
    tcp_server = start_tcp_server('0.0.0.0', 8765)

    await asyncio.gather(
        websocket_server,
        tcp_server
    )

if _name_ == "_main_":
    asyncio.run(main())
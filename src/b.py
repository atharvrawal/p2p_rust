import stun

def get_public_ip():
    nat_type, external_ip, external_port = stun.get_ip_info(stun_host="stun.l.google.com", stun_port=19302)
    print(f"Public IP: {external_ip}")
    print(f"Public Port: {external_port}")
    print(f"NAT Type: {nat_type}")

get_public_ip()


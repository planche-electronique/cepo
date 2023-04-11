import threading
import socket

target= 'localhost'
port = 7878

def requete():
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.connect((target, port))
    s.sendto(("GET /" + target + " HTTP/1.1\r\n").encode('ascii'), (target, port))
    s.sendto(("Host: " + '127.0.0.1' + "\r\n\r\n").encode('ascii'), (target, port))
    s.close()

threads = []

for i in range(100) :
    thread = threading.Thread(target=requete())
    thread.start()






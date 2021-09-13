import math
import os
import socket
import struct
import seqfold


class Worker(object):
    DEFAULT_TEMP = 25
    MAX_BUFF_SIZE = 2 ** 12

    def __init__(self, port):
        self.is_connected = False
        self.port = port
        self.channel: socket.socket = None
        self.listener = None

    def start(self):
        self.connect()
        self.loop()

    def connect(self):
        self.listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.listener.bind(("localhost", self.port))
        self.listener.listen(1)
        #self.listener.settimeout(0.1)
        # print("listening..")
        self.channel, _ = self.listener.accept()
        self.is_connected = True
        # print("socket connected")

    def loop(self):
        try:
            while self.is_connected:
                # print("looping")
                seq, temp = self.parse_params()
                if seq and temp:
                    self.handle_request(seq, temp)
                else:
                    self.restart()
        except:
            self.restart()

    def close(self):
        try:
            self.is_connected = False
            self.listener.shutdown(socket.SHUT_RDWR)
            self.listener.close()
            self.channel.close()
        except:
            pass

    def restart(self):
        self.close()
        self.connect()

    def send_dg(self, dg):
        print("sending dg=", dg, "process id=", os.getpid())
        self.channel.send(bytearray(struct.pack("f", dg)))

    def handle_request(self, seq, temp):
        try:
            dg = sum([0.5 * s.e if s.desc.startswith("STACK") else s.e for s in seqfold.fold(seq, temp)])
            if math.isinf(dg):
                self.send_dg(float(0))
            else:
                self.send_dg(dg)
        except:
            self.send_dg(float(0))

    def parse_params(self):
        try:
            received_bytes = self.channel.recv(Worker.MAX_BUFF_SIZE)
            received = str(received_bytes)
            index = -1
            try:
                index = received.index(",")
                temp = float(received[index + 1: -1])
            except:
                temp = Worker.DEFAULT_TEMP

            try:
                seq = received[2: index]
            except:
                return None, None

            return seq, temp
        except:
            return None, None

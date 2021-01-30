# -*- coding: utf-8 -*-

import time
import queue

queue = queue.Queue()

#from twisted.protocols.telnet import *
from twisted.protocols import basic
from objs.player import Player
from include.define import *

class Client(basic.LineOnlyReceiver):

    players = []
    delimiters = ['\r\n', '\r\000']
    MAX_LENGTH = 256

    def connectionMade(self):
        #from object import Player
        self.player = Player()

        self.player.idle = time.time()
        self.player.state = INACTIVE
        self.player.channel = self
        self.players.append(self.player)
        #print self.transport.getPeer().host
        self.player.welcome()
        #self.transport.write(IAC+WONT+ECHO)
        
    def write(self, line):
        return self.transport.write(line.encode("utf-8"))

    def dataReceived(self, chunk):
        self.player.idle = time.time()
        self._buffer = self._buffer + chunk

        for delim in self.delimiters:
            idx = self._buffer.find(delim.encode("utf-8"))
            if idx != -1:
                break
            
        while idx != -1:
            buf, self._buffer = self._buffer[:idx], self._buffer[idx+2:]
            self.lineReceived(buf)
            
            for delim in self.delimiters:
                idx = self._buffer.find(delim.encode("utf-8"))
                if idx != -1:
                    break
                    
    #def processLine(self, line):
    def lineReceived(self, line):
        self.player.process_input(line.decode("utf-8"), *self.player.process_input_args)
        #if self.player.state == ACTIVE:
        self.player.prompt(True)

    def connectionLost(self, reason):
        if self.player.state == ACTIVE:
            if self.player.env != None and self.player in self.player.env.objs:
                self.player.env.remove(self.player)
            self.player.save()
            self.player.logout()
        if self.player in self.players:
            self.players.remove(self.player)
        #self.player.clear()
        #del self.player.attr
        self.player.clearItems()
        self.player.input_to(None)
        if self.player.autoscript != None:
            self.player.autoscript.player = None
            del self.player.autoscript
        self.player.channel = None
        import sys
        #print sys.getrefcount(self.player)
        
        del self.player
        self.player = None

    def sendToAll(self, line, ex = None, noPrompt = False):
        for user in self.players:
            if user.state == ACTIVE and user != ex:
                user.sendLine('\r\n' + line)
                if noPrompt == False:
                    user.lpPrompt()
                    
    def sendToAll1(self, line, ex = None, noPrompt = False):
        for user in self.players:
            if user.state == ACTIVE and user != ex:
                user.write('\r\n' + line)
                if noPrompt == False:
                    user.lpPrompt()
    
    def sendToAllInOut(self, line, ex = None):
        for user in self.players:
            if user.state == ACTIVE and user.checkConfig('입출입메세지거부') == False and user != ex:
                user.sendLine('\r\n' + line)
                user.lpPrompt()
                
    def echoON(self):
        self.transport.write(IAC+WONT+ECHO)
        

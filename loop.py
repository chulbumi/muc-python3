# -*- coding: utf-8 -*-

import os
import copy
import sys
import time
import gc
import traceback

from twisted.internet import reactor
from client import Client
from include.define import *
from objs.mob import Mob
from objs.room import Room
from objs.player import Player
from objs.item import Item

class Loop:

    zoneList = []

    def __init__(self):
        self.cnt = 0
        self.run()

    def run(self):
        self.cnt += 1
        if self.cnt >= 60:
            self.cnt = 0
            #i = gc.collect()
            #if i != 0:
            #    print 'gc.collect %d' % i
            self.updateZones()
        t1 = time.time()
        rooms = []
        players = copy.copy(Client.players)
        try:
            for player in players:
                if player.state == INACTIVE and t1 - player.idle >= 10:
                    player.sendLine('\r\n\r\n입력 제한시간을 초과하였습니다.\r\n')
                    player.channel.transport.loseConnection()
                    continue
                if player.state != INACTIVE and t1 - player.idle >= 180:
                    player.sendLine('\r\n\r\n3분 동안 입력이 없어 접속을 종료합니다.\r\n')
                    player.channel.transport.loseConnection()
                    continue
                if player.state == ACTIVE:
                    player.update()
                if player.env != None and player.env not in rooms:
                    rooms.append(player.env)
        except:
            traceback.print_exc(file=sys.stderr)
            print(player['이름'])
        
        if len(Client.players) != 0:
            self.updateRooms(rooms)
            self.updateMovings()
            
        
        #print('t1 = %f'%t1)
        #time.sleep(2.5)
        t2 = time.time()
        #print('t2 = %f'%t2)
        dt = t2 - t1
        
        if 1 - dt < 0:
            dt = 1
        reactor.callLater(1 - dt, self.run)
        
    def updateZones(self):
        if len(self.zoneList) == 0:
            self.zoneList = list(Room.Zones.keys())
        zoneName = self.zoneList.pop()
        zone = Room.Zones[zoneName]
        try:
            for roomName in zone:
                room = zone[roomName]
                room.update()
        except:
            traceback.print_exc(file=sys.stderr)
   
    def updateRooms(self, rooms):
        try:
            for room in rooms:
                room.update()
        except:
            traceback.print_exc(file=sys.stderr)
    
    def updateMovings(self):
        if Mob.numMovings != 0:
            mob = Mob.movingMobs[0]
            mob.updateMoving()


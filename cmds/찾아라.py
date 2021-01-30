# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 2000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 2000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        from objs.room import Room, getRoom
        for zoneName in Room.Zones:
            zone = Room.Zones[zoneName]
            for roomName in zone:
                room = zone[roomName]
                for obj in room.objs:
                    if obj['이름'] == line:
                        ob.sendLine(room['이름'])



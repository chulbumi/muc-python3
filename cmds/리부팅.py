# -*- coding: utf-8 -*-

class CmdObj(Command):
    level = 1000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 1000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        from twisted.internet import reactor
        self.updateZones()
        reactor.stop()

    def updateZones(self):
        for zoneName in Room.Zones:
            print("update zones..." + zoneName)
            zone = Room.Zones[zoneName]
            for roomName in zone:
                room = zone[roomName]
                room.update()

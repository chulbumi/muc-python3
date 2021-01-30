# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    level = 2000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 2000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
            
        from client import Client
        for ply in Client.players:
            if ply.state != ACTIVE:
                continue
            ply.save()

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
            if ply.env == ob.env:
                ob.sendLine('☞ 같은 곳이에요. ^^')
                continue
            if ply.enterRoom(ob.env, '소환', '소환') == False:
                ob.sendLine('☞ 공간이동에 실패하였습니다.')
            ply.clearTarget()
            ply.lpPrompt()

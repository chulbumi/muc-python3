# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 1000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 1000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        if line == '':
            ob.sendLine('☞ 사용법: [사용자명] 정리')
            return
            
        ob.sendLine(line)
        slist = []
        for ply in ob.channel.players:
            if ply['이름'] == line:
                ply.do_command('끝')
                ply.channel.transport.loseConnection()
                ob.channel.players.remove(ply)
                ob.sendLine('☞ 정리하였습니다. *^_^*')
                return

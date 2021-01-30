# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 1000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 1000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        if line == '':
            ob.sendLine('☞ 사용법: [대상] 사용자몹소환')
            return
        ply = Player()
        if ply.load(line) == False:
            ob.sendLine('존재하지않는 사용자입니다.')
            return
        ply.state = ACTIVE
        ob.env.insert(ply)
        ob.channel.players.append(ply)
        ob.sendLine('%s 소환하였습니다.' % ply.han_obj())

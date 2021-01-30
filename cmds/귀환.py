# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        if ob.env.checkAttr('귀환금지'):
            ob.sendLine('☞ 이곳에선 귀환하실 수 없어요. ^^')
            return
        # check if player can't move
        if ob.isMovable() == False:
            ob.sendLine('☞ 지금은 귀환할 상황이 아니에요. ^^')
            return

        ret = ob.get('귀환지맵')
        if ret == '':
            room = getRoom('낙양성:42')
        else:
            room = getRoom(ret)
        if room == None:
            ob.sendLine('귀환지맵이 없습니다. 관리자에게 연락하세요.')
            return
        if room == ob.env:
            ob.sendLine('☞ 같은 자리에요. ^^')
            return
        ob.enterRoom(room, '귀환', '귀환')



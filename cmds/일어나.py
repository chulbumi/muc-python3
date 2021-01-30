# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        if ob.act == ACT_REST:
            ob.sendLine('당신이 운기조식을 마치며 벌떡 일어섭니다.')
            ob.sendRoom('%s 운기조식을 마치며 벌떡 일어섭니다.' % ob.han_iga())
            ob.act = ACT_STAND
        elif ob.act == ACT_STAND:
            ob.sendLine('☞ 이미 서있는 상태입니다. ^^')
        else:
            ob.sendLine('☞ 운기조식중인 상태가 아닙니다.')

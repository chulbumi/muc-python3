# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        if ob.env.checkAttr('쉼금지'):
            ob.sendLine('☞ 이곳에서 운기조식하기엔 적당하지 않군요.')
            return
        if ob.act == ACT_REST:
            ob.sendLine('☞ 벌써 쉬고 있어요. ^^')
        elif ob.act == ACT_STAND:
            ob.sendLine('당신이 자세를 편안히 하며 운기조식에 들어갑니다.')
            ob.sendRoom('%s 자세를 편안히 하며 운기조식에 들어갑니다.' % ob.han_iga())
            ob.act = ACT_REST
        else:
            ob.sendLine('☞ 지금 쉬기에는 좋지가 않네요. ^^')

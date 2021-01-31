# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        mob = ob.env.findMerchant()
        if mob == None:
            ob.sendLine('☞ 품목을 보여줄 상인이 없어요. ^^')
            return
        if mob['물건판매스크립'] == '':
            ob.sendLine('☞ 품목을 보여줄 상인이 없어요. ^^')
            return
        desc = mob['물건판매스크립']

        if type(desc) == list:
            for l in desc:
                ob.sendLine(l)
        else:
            ob.sendLine(desc)



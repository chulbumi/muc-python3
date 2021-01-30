# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        import pickle
        import uuid

        if line == '':
            ob.sendLine('☞ 사용법: [물품] 반납')
            return
        num = getInt(line)

        mob = ob.env.findObjName('진영')
        if mob == None:
            ob.sendLine('☞ 이곳에서는 불가능해요.')
            return

        name, order = getNameOrder(line)
        item = ob.findObjInven(name, order)
        if item == None:
            ob.sendLine('☞ 그런 아이템이 소지품에 없어요.')
            return

        if item['고유번호'] == None or item['고유번호'] == '':
            ob.sendLine('☞ 반납 가능한 물품이 아닙니다.')
            return

        try:
            with open("data/config/book.dat", "rb") as fr:
                data = pickle.load(fr)
        except:
            ob.sendLine('☞ 반납 가능한 물품이 없습니다.')
            return

        if len(data) == 0:
            ob.sendLine('☞ 반납 가능한 물품이 없습니다.')
            return

        for itm in data:
            match = False
            if type(item['고유번호']) == type(''):
                if itm['고유번호'] == uuid.UUID(item['고유번호']):
                    match = True
            else:
                if itm['고유번호'] == item['고유번호']:
                    match = True
            if match == True:
                itm["대여가능"] = True
                itm["대여"] =''
                ob.remove(item)

                with open("data/config/book.dat", "wb") as fw:
                    pickle.dump(data, fw)
                ob.sendLine('☞ 반납이 완료 되었습니다.')
                return

        ob.sendLine('☞ 반납 할 수 없습니다.')
